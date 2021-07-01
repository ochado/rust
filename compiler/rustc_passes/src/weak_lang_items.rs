//! Validity checking for weak lang items

use rustc_data_structures::fx::FxHashSet;
use rustc_errors::struct_span_err;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, NestedVisitorMap, Visitor};
use rustc_hir::lang_items::{self, LangItem};
use rustc_hir::weak_lang_items::WEAK_ITEMS_REFS;
use rustc_middle::middle::lang_items::required;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::CrateType;
use rustc_span::symbol::Symbol;
use rustc_span::Span;

struct Context<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    items: &'a mut lang_items::LanguageItems,
}

/// Checks the crate for usage of weak lang items, returning a vector of all the
/// language items required by this crate, but not defined yet.
pub fn check_crate<'tcx>(tcx: TyCtxt<'tcx>, items: &mut lang_items::LanguageItems) {
    // These are never called by user code, they're generated by the compiler.
    // They will never implicitly be added to the `missing` array unless we do
    // so here.
    if items.eh_personality().is_none() {
        items.missing.push(LangItem::EhPersonality);
    }
    if tcx.sess.target.is_like_emscripten && items.eh_catch_typeinfo().is_none() {
        items.missing.push(LangItem::EhCatchTypeinfo);
    }

    {
        let mut cx = Context { tcx, items };
        tcx.hir().krate().visit_all_item_likes(&mut cx.as_deep_visitor());
    }
    verify(tcx, items);
}

fn verify<'tcx>(tcx: TyCtxt<'tcx>, items: &lang_items::LanguageItems) {
    // We only need to check for the presence of weak lang items if we're
    // emitting something that's not an rlib.
    let needs_check = tcx.sess.crate_types().iter().any(|kind| match *kind {
        CrateType::Dylib
        | CrateType::ProcMacro
        | CrateType::Cdylib
        | CrateType::Executable
        | CrateType::Staticlib => true,
        CrateType::Rlib => false,
    });
    if !needs_check {
        return;
    }

    let mut missing = FxHashSet::default();
    for &cnum in tcx.crates(()).iter() {
        for &item in tcx.missing_lang_items(cnum).iter() {
            missing.insert(item);
        }
    }

    for (name, item) in WEAK_ITEMS_REFS.clone().into_sorted_vector().into_iter() {
        if missing.contains(&item) && required(tcx, item) && items.require(item).is_err() {
            if item == LangItem::PanicImpl {
                tcx.sess.err("`#[panic_handler]` function required, but not found");
            } else if item == LangItem::Oom {
                if !tcx.features().default_alloc_error_handler {
                    tcx.sess.err("`#[alloc_error_handler]` function required, but not found.");
                    tcx.sess.note_without_error("Use `#![feature(default_alloc_error_handler)]` for a default error handler.");
                }
            } else {
                tcx.sess.err(&format!("language item required, but not found: `{}`", name));
            }
        }
    }
}

impl<'a, 'tcx> Context<'a, 'tcx> {
    fn register(&mut self, name: Symbol, span: Span) {
        if let Some(&item) = WEAK_ITEMS_REFS.get(&name) {
            if self.items.require(item).is_err() {
                self.items.missing.push(item);
            }
        } else {
            struct_span_err!(self.tcx.sess, span, E0264, "unknown external lang item: `{}`", name)
                .emit();
        }
    }
}

impl<'a, 'tcx, 'v> Visitor<'v> for Context<'a, 'tcx> {
    type Map = intravisit::ErasedMap<'v>;

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::None
    }

    fn visit_foreign_item(&mut self, i: &hir::ForeignItem<'_>) {
        let check_name = |attr, sym| self.tcx.sess.check_name(attr, sym);
        let attrs = self.tcx.hir().attrs(i.hir_id());
        if let Some((lang_item, _)) = lang_items::extract(check_name, attrs) {
            self.register(lang_item, i.span);
        }
        intravisit::walk_foreign_item(self, i)
    }
}
