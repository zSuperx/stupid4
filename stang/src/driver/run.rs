use crate::IRs::hir::HirObj;
use crate::driver::args::*;
use crate::parser::parse_file;
use crate::translation_unit::TranslationUnit;
use std::sync::LazyLock;

pub static CFG: LazyLock<Config> = LazyLock::new(validate_config);

pub fn run() {
    let mut tu = TranslationUnit::new();
    let objects = parse_file(&CFG.input);
    tu.resolve_top_level(&objects);

    let mut objs = vec![];
    for obj in objects {
        match obj.inner {
            HirObj::Fn(hir_function) => {
                objs.push(tu.check_func(hir_function));
            }
            _ => {}
        }
    }

    for obj in objs {
        let mut x = tu.codegen_func(obj);
        x.print(CFG.verbose);
    }
}
