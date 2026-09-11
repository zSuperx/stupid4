use std::collections::HashMap;

use super::*;
use crate::ast::*;
use crate::common::*;
use crate::translation_unit::LoopLabels;
use crate::translation_unit::SymbolInfo;
use crate::translation_unit::SymbolKind;
use crate::translation_unit::global_state;
use crate::translation_unit::next_symbol;

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

    pub symbol_counter: usize,

    pub body: Option<TirStmt>,
}

impl TirFunction {
    pub fn add_local_symbol(
        &mut self,
        name: Spanned<&'static str>,
        ty: TypeId,
        kind: SymbolKind,
    ) -> Symbol {
        let symbol = next_symbol(name.inner);
        self.env.insert(name.inner, symbol);
        self.symbol_table.insert(
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
}
