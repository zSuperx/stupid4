use shrimple::stir::builder::IRModule;
use shrimple::x86Function;

use crate::IRs::tir::TirFunction;
use crate::ast::QualType;
use crate::common::{Scopes, Spanned};
use crate::die;
use crate::driver::args::*;
use crate::parser::parse_file;
use crate::sema::CompilerContext;
use crate::translation_unit::{add_str, global_state, next_symbol, qtype};
use std::collections::HashSet;
use std::io::Write;
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
        gs.type_names.insert(add_str(ty.to_string()), qtype(&ty));
    }
}

pub fn run() {
    init_state();

    let mut ctx = CompilerContext {
        scopes: Scopes::new(),
        loop_depth: 0,
        loop_labels: Vec::new(),
        symbol_table: Default::default(),
        top_level_scope: Default::default(),
        module: IRModule::new(),
    };

    let program = parse_file(&CFG.input);
    ctx.resolve_top_level(&program);

    let mut tir_functions = vec![];

    for Spanned {
        inner: hir_function,
        ..
    } in program.functions
    {
        let tf = TirFunction {
            return_type: ctx.qualify_type(&hir_function.return_type.inner),
            symbol: ctx.resolve_ident(&hir_function.name.inner).unwrap(),
            name: hir_function.name.clone(),
            local_symbols: HashSet::new(),
            body: None,
        };

        ctx.reset();
        let tir_function = hir_function.type_check(&mut ctx);
        tir_functions.push(tir_function);
    }

    for tir_function in tir_functions.iter() {
        ctx.module.add_symbol(tir_function.name.inner.to_string());
    }

    for tir_function in tir_functions {
        let ir_function = tir_function.codegen(&mut ctx);
        ctx.module.add_function(ir_function);
    }

    let mut writer: Box<dyn Write> = match CFG.output.as_str() {
        "-" => Box::new(std::io::stdout()),
        file => {
            Box::new(std::fs::File::create(file).unwrap_or_else(|_| die!("Could not open {file}")))
        }
    };

    match CFG.action {
        Action::EmitIr => {
            for function in ctx.module.functions() {
                function.print(&mut writer, CFG.verbose);
            }
        }
        Action::EmitAsm => {
            for function in ctx.module.functions() {
                let target_function = x86Function::lower(function);
                target_function.print(&mut writer, CFG.verbose);
            }
        }
        Action::CompileOnly => die!("TODO: Compile"),
        Action::AssembleAndLink => die!("TODO: Assemble + Link"),
    }
}
