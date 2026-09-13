use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use super::*;
use crate::ast::*;
use crate::common::*;
use crate::translation_unit::{
    LoopLabels, Symbol, SymbolInfo, SymbolKind, global_state, next_symbol,
};

#[derive(Debug, Clone)]
pub enum TirObj {
    Fn {
        symbol: Symbol,
        returns: Rc<QualType>,
        args: Vec<(Symbol, Rc<QualType>)>,
        body: Box<TirStmt>,
    },
    Global {
        lhs: Symbol,
        rhs: Box<TirExprKind>,
    },
    Struct {
        name: Symbol,
        fields: Vec<(Symbol, Rc<QualType>)>,
    },
}

#[derive(Debug)]
pub struct TirFunction {
    pub name: Spanned<RcString>,
    pub symbol: Symbol,
    pub return_type: Rc<QualType>,
    pub local_symbols: HashSet<Symbol>,
    pub body: Option<TirStmt>,
}
