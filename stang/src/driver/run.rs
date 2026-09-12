use shrimple::stir::builder::IRModule;

use crate::IRs::hir::HirObj;
use crate::ast::QualType;
use crate::driver::args::*;
use crate::parser::parse_file;
use crate::translation_unit::{TranslationUnit, add_str, qtype, global_state};
use std::sync::LazyLock;

pub static CFG: LazyLock<Config> = LazyLock::new(validate_config);

fn init_state() {
    let gs = global_state();

    #[rustfmt::skip]
    let builtin_types = [
        QualType::I8,   QualType::U8,
        QualType::I16,  QualType::U16,
        QualType::I32,  QualType::U32,
        QualType::I64,  QualType::U64,
        QualType::Bool, QualType::Void
    ];

    for ty in builtin_types {
        gs.type_names
            .insert(add_str(&ty.to_string()), qtype(&ty));
    }
}

pub fn run() {
    init_state();

    let mut tu = TranslationUnit::new();
    let objects = parse_file(&CFG.input);
    tu.resolve_top_level(&objects);

    let mut objs = vec![];
    for obj in objects {
        match obj.inner {
            HirObj::Fn(hir_function) => {
                let tf = hir_function.type_check(&mut tu);
                objs.push(tf);
            }
            _ => {}
        }
    }

    let mut builder = IRModule::new();

    for obj in objs {
        obj.codegen(&mut builder);
    }

    for function in builder.functions() {
        function.print(CFG.verbose);
        println!()
    }
}
