use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, OnceLock};

use crate::IRs::hir::*;
use crate::IRs::tir::{TirExpr, TirExprKind, TirFunction, TirStmt};
use crate::common::*;
use crate::parser::ParsedProgram;
use crate::sema::CompilerContext;
use std::cell::RefCell;
use std::rc::Rc;

use interner::Pool;
use shrimple::stir::builder::IRLabel;
use shrimple::stir::isa::IRValue;

use crate::ast::*;

#[derive(Default)]
pub struct GlobalState {
    pub type_names: HashMap<RcString, Rc<QualType>>,

    pub qualified_types: Pool<QualType>,
    pub raw_types: Pool<RawType>,

    pub symbols: Pool<String>,
    pub symbol_counter: usize,
}

#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Debug)]
pub struct Symbol(Rc<String>, usize);

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}.{}", self.0, self.1))
    }
}

pub fn next_symbol(name: &str) -> Symbol {
    let s = add_str(name);
    let state = global_state();
    state.symbol_counter += 1;
    Symbol(s, state.symbol_counter - 1)
}

impl CompilerContext {
    /// Reads all parsed objects and registers global symbols and global types.
    ///
    /// Without this, the user would have to resort to C-style forward-declarations to refer to types
    /// that were declared physically later in the file.
    pub fn resolve_top_level(&mut self, program: &ParsedProgram) {
        for struct_ in program.structs.iter() {
            self.resolve_global_types(struct_);
        }

        for global in program.globals.iter() {
            self.resolve_global_variables(global);
        }

        for function in program.functions.iter() {
            self.resolve_global_functions(function);
        }
    }

    /// Resolves all types at the top-level scope. This is so future invocations of [`resolve_type`]
    /// actually have something to resolve to
    fn resolve_global_types(&mut self, Spanned { inner: struct_, .. }: &Spanned<HirStruct>) {
        let HirStruct { name, fields } = struct_;

        let s = qtype(&QualType::Struct {
            name: name.inner.clone(),
            fields: fields
                .iter()
                .map(|(n, t)| (n.inner.clone(), self.qualify_type(&t.inner)))
                .collect(),
        });
        global_state().type_names.insert(name.inner.clone(), s);
    }

    /// Resolves all functions at the top-level scope. This is what allows functions to know they
    /// are referring to one another.
    fn resolve_global_functions(
        &mut self,
        Spanned {
            inner: function, ..
        }: &Spanned<HirFunction>,
    ) {
        let HirFunction {
            name,
            return_type: returns,
            args,
            ..
        } = function;
        let mut arg_types = vec![];
        for (_, ty) in args.iter() {
            let resolved_ty = self.qualify_type(&ty.inner);
            arg_types.push(resolved_ty);
        }
        let return_ty = self.qualify_type(&returns.inner);
        let function_ty = QualType::Function {
            arg_types,
            return_type: return_ty,
        };
        let ty = qtype(&function_ty);
        let symbol = next_symbol(&name.inner);
        let module_symbol = self.module.add_symbol(name.inner.to_string());
        self.top_level_scope
            .insert(name.inner.clone(), symbol.clone());
        self.symbol_table.insert(
            symbol.clone(),
            SymbolInfo {
                symbol,
                raw_name: name.clone(),
                ty,
                kind: SymbolKind::Function,
                address_taken: false,
                value: Some(IRValue::Sym(module_symbol)),
            },
        );
    }

    fn resolve_global_variables(
        &mut self,
        Spanned {
            inner: variable, ..
        }: &Spanned<HirGlobal>,
    ) {
        // TODO: implement this
    }

    /// Resolves a Type::Unresolved(..) into a Type
    pub fn qualify_type(&mut self, ty: &RawType) -> Rc<QualType> {
        match ty {
            RawType::Base(name) => {
                let Some(qtype) = global_state().type_names.get(name) else {
                    die!("Unknown type {ty}")
                };
                qtype.clone()
            }
            RawType::Pointer(inner) => {
                let inner_ty = self.qualify_type(inner);
                qtype(&QualType::Pointer(inner_ty))
            }
            RawType::Function {
                arg_types,
                return_type,
            } => {
                let ty = QualType::Function {
                    arg_types: arg_types.iter().map(|t| self.qualify_type(t)).collect(),
                    return_type: self.qualify_type(return_type),
                };
                qtype(&ty)
            }
        }
    }
}

pub static mut GLOBAL_STATE: LazyLock<GlobalState> = LazyLock::new(GlobalState::new);
pub static SOURCE: OnceLock<Vec<u8>> = OnceLock::new();
pub static mut STRINGS: LazyLock<Pool<String>> = LazyLock::new(Pool::new);

pub fn source() -> &'static Vec<u8> {
    SOURCE.get().unwrap()
}

pub fn global_state() -> &'static mut GlobalState {
    unsafe { &mut GLOBAL_STATE }
}

pub fn add_str<T: AsRef<str>>(value: T) -> Rc<String> {
    unsafe { STRINGS.get(&value.as_ref().to_string()) }
}

pub fn rtype(ty: &RawType) -> Rc<RawType> {
    global_state().raw_types.get(ty)
}

pub fn qtype(ty: &QualType) -> Rc<QualType> {
    global_state().qualified_types.get(ty)
}

impl GlobalState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// `continue` jumps to `cond_block` and `break` jumps to `end_block`
#[derive(Debug, Clone, Copy)]
pub struct LoopLabels {
    pub cond_block: IRLabel,
    pub end_block: IRLabel,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SymbolKind {
    Local,
    Arg(usize),
    Global,
    Function,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub symbol: Symbol,
    pub raw_name: Spanned<RcString>,
    pub ty: Rc<QualType>,
    pub kind: SymbolKind,

    /// Indicates whether the user captured the address of this variable using the & operator
    /// This is used to prevent PHI promotion, and leaves the slot as an addressable location
    pub address_taken: bool,

    pub value: Option<IRValue>,
}

impl SymbolInfo {
    /// Creates an AddrOf Typed AST node. This is useful for ad-hoc AST transformations
    pub fn create_addr_of(&self) -> TirExpr {
        let ty = self.ty.clone();
        let kind = TirExprKind::AddrOf(self.symbol.clone());
        TirExpr::new(kind, ty)
    }
}
