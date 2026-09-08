use std::collections::HashMap;

use super::*;
use crate::ast::*;
use crate::common::*;
use crate::translation_unit::SymbolInfo;

#[derive(Debug, Clone)]
pub enum TirObj {
    Fn {
        symbol: Symbol,
        returns: TypeId,
        args: Vec<(Symbol, TypeId)>,
        body: Box<TirStmt>,
    },
    Global {
        lhs: Symbol,
        rhs: Box<TirExprKind>,
    },
    Struct {
        name: Symbol,
        fields: Vec<(Symbol, TypeId)>,
    },
}

#[derive(Debug)]
pub struct TirFunction {
    /// The raw name of the function
    pub name: Spanned<&'static str>,

    /// The symbol its mapped to
    pub symbol: Symbol,
    
    /// Local symbol table
    pub symbol_table: HashMap<Symbol, SymbolInfo>,

    /// The return type of this function.
    pub return_type: TypeId,


    /// The AST node of the function representing the body
    pub body: TirStmt,
}
