use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, OnceLock};

use crate::IRs::hir::{HirFunction, HirObj};
use crate::common::*;
use std::cell::RefCell;
use std::rc::Rc;

use registry::Registry;
use shrimple::stir::builder::IRBB;
use shrimple::stir::isa::IRValue;

use crate::ast::*;

#[derive(Default)]
pub struct GlobalState {
    pub type_names: HashMap<&'static str, TypeId>,

    /// Uniquely ID'd resolved types.
    pub types: Rc<RefCell<Registry<Type>>>,

    /// Uniquely ID'd scoped identifiers
    pub symbols: Rc<RefCell<Registry<String>>>,

    /// Distinguishes shadowed vars
    pub symbol_counter: usize,
}

#[derive(Debug, Default)]
pub struct FunctionContext {
    pub symbol: Option<Symbol>,

    /// Tracks string -> symbol mappings. Looking up a symbol by its string name starts at the inner
    /// most (current) scope, going up in scopes on failure
    pub env: Env<&'static str, Symbol>,

    /// Used to codegen continue/break
    pub loop_labels: Vec<LoopLabels>,

    /// Use in sema to validate use of continue/break
    pub loop_depth: usize,

    pub return_type: Option<TypeId>,

    pub symbol_table: HashMap<Symbol, SymbolInfo>,
}

#[derive(Debug, Default)]
pub struct TranslationUnit {
    /// Global string to symbol map
    ///
    /// Contains names of functions and global variables
    top_level_scope: HashMap<&'static str, Symbol>,

    symbol_table: HashMap<Symbol, SymbolInfo>,

    pub current_function: FunctionContext,
}

impl TranslationUnit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_local_symbol(
        &mut self,
        name: Spanned<&'static str>,
        ty: TypeId,
        kind: SymbolKind,
    ) -> Symbol {
        let symbol = next_symbol(name.inner);
        self.current_function.env.insert(name.inner, symbol);
        self.current_function.symbol_table.insert(
            symbol,
            SymbolInfo {
                symbol,
                raw_name: name,
                ty,
                kind,
                address_taken: Default::default(),
                value: Default::default(),
            },
        );
        symbol
    }

    pub fn lookup_symbol(&mut self, symbol: Symbol) -> &SymbolInfo {
        match self.current_function.symbol_table.get(&symbol) {
            Some(i) => i,
            None => die!("Symbol not found: {symbol}"),
        }
    }

    /// Resets all compiler context regarding the current function.
    pub fn reset_function_state(&mut self, symbol: Symbol, return_type: TypeId) {
        self.current_function = FunctionContext {
            symbol: Some(symbol),
            return_type: Some(return_type),
            ..Default::default()
        };
        self.current_function
            .env
            .push_filled_scope(self.top_level_scope.clone());
    }

    pub fn current_function_name(&self) -> Spanned<&'static str> {
        self.symbol_table
            .get(self.current_function.symbol.as_ref().unwrap())
            .unwrap()
            .raw_name
    }

    /// Reads all parsed objects and registers global symbols and global types.
    ///
    /// Without this, the user would have to resort to C-style forward-declarations to refer to types
    /// that were declared physically later in the file.
    pub fn resolve_top_level(&mut self, objects: &Vec<Spanned<HirObj>>) {
        for obj in objects {
            self.resolve_global_types(obj);
        }

        for obj in objects {
            self.resolve_global_names(obj);
        }
    }

    /// Resolves a Type::Unresolved(..) into a Type
    pub fn resolve_type(&mut self, s @ Spanned { inner: ty, span }: &Spanned<TypeId>) -> TypeId {
        match ty.lookup() {
            Type::Unresolved(name) => {
                let Some(id) = global_state().type_names.get(name) else {
                    die!("Unknown type {s}")
                };
                *id
            }
            Type::Function { .. } => todo!(),
            Type::Pointer(id) => {
                let inner_ty = self.resolve_type(&Spanned::new(*id, *span));
                add_type(Type::Pointer(inner_ty))
            }
            _ => s.inner,
        }
    }

    /// Resolves all types at the top-level scope. This is so future invocations of [`resolve_type`]
    /// actually have something to resolve to
    fn resolve_global_types(&mut self, Spanned { inner: obj, .. }: &Spanned<HirObj>) {
        match obj {
            HirObj::Fn(..) => {}
            HirObj::Global { .. } => {}
            HirObj::Struct { name, fields } => {
                let s = add_type(Type::Base {
                    name: name.inner,
                    fields: fields
                        .iter()
                        .map(|(n, t)| (n.inner, self.resolve_type(t)))
                        .collect(),
                });
                global_state().type_names.insert(name.inner, s);
            }
        }
    }

    /// Resolves all identifiers at the top-level scope. This includes functions and global variables,
    /// and is what allows them to know they are referring to one another.
    fn resolve_global_names(&mut self, Spanned { inner: obj, .. }: &Spanned<HirObj>) {
        match obj {
            HirObj::Fn(HirFunction {
                name,
                returns,
                args,
                ..
            }) => {
                let mut arg_types = vec![];
                for (_, ty) in args.iter() {
                    let resolved_ty = self.resolve_type(ty);
                    arg_types.push(resolved_ty);
                }
                let return_ty = self.resolve_type(returns);
                let function_ty = Type::Function {
                    arg_tys: arg_types,
                    return_ty,
                };
                let ty = add_type(function_ty);
                let symbol = next_symbol(name.inner);
                self.top_level_scope.insert(name.inner, symbol);
                self.symbol_table.insert(
                    symbol,
                    SymbolInfo {
                        symbol,
                        raw_name: *name,
                        ty,
                        kind: SymbolKind::Function,
                        address_taken: false,
                        value: None,
                    },
                );
            }
            HirObj::Global { .. } => {
                todo!()
            }
            HirObj::Struct { .. } => {}
        }
    }
}

pub static mut GLOBAL_STATE: LazyLock<GlobalState> = LazyLock::new(GlobalState::new);
pub static SOURCE: OnceLock<Vec<u8>> = OnceLock::new();
pub static mut STRINGS: LazyLock<HashSet<String>> = LazyLock::new(HashSet::new);

pub fn source() -> &'static Vec<u8> {
    SOURCE.get().unwrap()
}

pub fn global_state() -> &'static mut GlobalState {
    unsafe { &mut GLOBAL_STATE }
}

pub fn add_str(value: &str) -> &'static str {
    unsafe {
        if let Some(s) = STRINGS.get(value) {
            s
        } else {
            STRINGS.insert(value.to_string());
            STRINGS.get(value).unwrap()
        }
    }
}

pub fn add_type(ty: Type) -> TypeId {
    global_state().types.borrow_mut().add(ty)
}

pub fn next_symbol(name: &str) -> Symbol {
    let state = global_state();
    let id = state.symbol_counter;
    state.symbol_counter += 1;
    state.symbols.borrow_mut().add(format!("{name}.{id}"))
}

impl GlobalState {
    pub fn new() -> Self {
        let mut s = Self::default();

        #[rustfmt::skip]
        let builtin_types = [
            Type::I8,   Type::U8,
            Type::I16,  Type::U16,
            Type::I32,  Type::U32,
            Type::I64,  Type::U64,
            Type::Bool, Type::Void
        ];

        for ty in builtin_types {
            let ty_str = ty.to_string().leak();
            let id = s.types.borrow_mut().add(ty);
            s.type_names.insert(ty_str, id);
        }
        s
    }
}

/// `continue` jumps to COND_BB and `break` jumps to END_BB
#[derive(Debug, Clone, Copy)]
pub struct LoopLabels {
    pub cond_block: IRBB,
    pub end_block: IRBB,
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
    pub raw_name: Spanned<&'static str>,
    pub ty: TypeId,
    pub kind: SymbolKind,
    pub address_taken: bool,
    pub value: Option<IRValue>,
}
