// revisions: min_tait full_tait
#![feature(min_type_alias_impl_trait)]
#![cfg_attr(full_tait, feature(type_alias_impl_trait))]
//[full_tait]~^ WARN incomplete
// build-pass (FIXME(62277): could be check-pass?)

trait Bar {}
struct Dummy;
impl Bar for Dummy {}

trait Foo {
    type Assoc: Bar;
    fn foo() -> Self::Assoc;
    fn bar() -> Self::Assoc;
}

type Helper = impl Bar;

impl Foo for i32 {
    type Assoc = Helper;
    fn foo() -> Helper {
        Dummy
    }
    fn bar() -> Helper {
        Dummy
    }
}

fn main() {}
