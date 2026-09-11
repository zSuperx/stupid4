use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, OnceLock};

use crate::IRs::hir::{HirFunction, HirObj};
use crate::IRs::tir::{TirFunction, TirStmt};
use crate::common::*;
use std::cell::RefCell;
use std::rc::Rc;

use registry::Registry;
use shrimple::stir::builder::IRLabel;
use shrimple::stir::isa::IRValue;

use crate::ast::*;

#[derive(Default)]
pub struct GlobalState {
    pub type_names: HashMap<&'static str, TypeId>,

    /// Uniquely ID'd resolved types.
    pub types: Rc<RefCell<Registry<Type>>>,

    /// Uniquely ID'd scoped identifiers
    pub symbols: Rc<RefCell<Registry<String>>>,

    pub symbol_counter: usize,
}

#[derive(Debug)]
pub struct FunctionContext {
    pub name: Spanned<&'static str>,

    pub symbol: Symbol,

    pub return_type: TypeId,

    /// Tracks string -> symbol mappings. Looking up a symbol by its string name starts at the inner
    /// most (current) scope, going up in scopes on failure
    pub env: Env<&'static str, Symbol>,

    /// Used to codegen continue/break
    pub loop_labels: Vec<LoopLabels>,

    /// Use in sema to validate use of continue/break
    pub loop_depth: usize,

    pub symbol_table: HashMap<Symbol, SymbolInfo>,

    pub body: Option<TirStmt>,
}

#[derive(Debug, Default)]
pub struct TranslationUnit {
    /// Global string to symbol map
    ///
    /// Contains names of functions and global variables
    pub top_level_scope: HashMap<&'static str, Symbol>,

    pub global_symbol_table: HashMap<Symbol, SymbolInfo>,

    /// Distinguishes shadowed vars
    symbol_counter: usize,
}

pub fn next_symbol(name: &'static str) -> Symbol {
    let state = global_state();
    state.symbols.borrow_mut().add(format!("{name}.{}", {
        state.symbol_counter += 1;
        state.symbol_counter - 1
    }))
}

impl TranslationUnit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_function(&mut self) {
        todo!()
    }

    // pub fn lookup_symbol(&mut self, symbol: Symbol) -> &SymbolInfo {
    //     match self.current_function.symbol_table.get(&symbol) {
    //         Some(i) => i,
    //         None => die!("Symbol not found: {symbol}"),
    //     }
    // }
    //

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
    pub fn resolve_type(&mut self, ty: TypeId) -> TypeId {
        match ty.lookup() {
            Type::Unresolved(name) => {
                let Some(id) = global_state().type_names.get(name) else {
                    die!("Unknown type {ty}")
                };
                *id
            }
            Type::Function { .. } => todo!(),
            Type::Pointer(id) => {
                let inner_ty = self.resolve_type(*id);
                add_type(Type::Pointer(inner_ty))
            }
            _ => ty,
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
                        .map(|(n, t)| (n.inner, self.resolve_type(t.inner)))
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
                return_type: returns,
                args,
                ..
            }) => {
                let mut arg_types = vec![];
                for (_, ty) in args.iter() {
                    let resolved_ty = self.resolve_type(ty.inner);
                    arg_types.push(resolved_ty);
                }
                let return_ty = self.resolve_type(returns.inner);
                let function_ty = Type::Function {
                    arg_types,
                    return_type: return_ty,
                };
                let ty = add_type(function_ty);
                let symbol = next_symbol(name.inner);
                self.top_level_scope.insert(name.inner, symbol);
                self.global_symbol_table.insert(
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

pub fn lookup_ident<'a, 'b, 'c>(
    tu: &'a mut TranslationUnit,
    tf: &'b mut TirFunction,
    ident: &'static str,
) -> Option<&'c SymbolInfo>
where
    'a: 'c,
    'b: 'c,
{
    let Some(symbol) = tf.env.get(&ident) else {
        return None;
    };
    Some(lookup_symbol(tu, tf, symbol))
}

pub fn lookup_symbol<'a, 'b, 'c>(
    tu: &'a mut TranslationUnit,
    tf: &'b mut TirFunction,
    symbol: Symbol,
) -> &'c SymbolInfo
where
    'a: 'c,
    'b: 'c,
{
    match tf.symbol_table.get(&symbol) {
        Some(i) => i,
        None => {
            tu.global_symbol_table
                .get(&symbol)
                .expect("Could not find symbol: {symbol}")
        }
    }
}

/// `continue` jumps to COND_BB and `break` jumps to END_BB
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
    pub raw_name: Spanned<&'static str>,
    pub ty: TypeId,
    pub kind: SymbolKind,
    pub address_taken: bool,
    pub value: Option<IRValue>,
}
