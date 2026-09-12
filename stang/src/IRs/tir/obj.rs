use std::collections::HashMap;
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

    /// Tracks string -> symbol mappings. Looking up a symbol by its string name starts at the inner
    /// most (current) scope, going up in scopes on failure
    pub env: Env<RcString, Symbol>,

    /// Used to codegen continue/break
    pub loop_labels: Vec<LoopLabels>,

    /// Use in sema to validate use of continue/break
    pub loop_depth: usize,

    pub symbol_table: HashMap<Symbol, SymbolInfo>,

    pub symbol_counter: usize,

    pub body: Option<TirStmt>,
}

impl TirFunction {
    pub fn add_local_symbol(
        &mut self,
        name: Spanned<RcString>,
        ty: Rc<QualType>,
        kind: SymbolKind,
    ) -> Symbol {
        let symbol = next_symbol(&name.inner);
        self.env.insert(name.inner.clone(), symbol.clone());
        self.symbol_table.insert(
            symbol.clone(),
            SymbolInfo {
                symbol: symbol.clone(),
                raw_name: name,
                ty,
                kind,
                address_taken: Default::default(),
                value: Default::default(),
            },
        );
        symbol
    }
}
