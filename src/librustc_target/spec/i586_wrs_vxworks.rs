use crate::spec::TargetResult;

pub fn target() -> TargetResult {
    let mut base = super::i686_wrs_vxworks::target()?;
    base.options.cpu = "pentium".to_string();
    base.llvm_target = "i586-unknown-linux-gnu".to_string();
    Ok(base)
}
