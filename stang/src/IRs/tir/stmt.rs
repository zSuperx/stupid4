use super::*;
use crate::common::*;
use crate::translation_unit::Symbol;

#[derive(Debug, Clone)]
pub enum TirStmt {
    While {
        cond: TirExpr,
        body: Box<TirStmt>,
    },
    Continue,
    Break,
    If {
        cond: TirExpr,
        then_: Box<TirStmt>,
        else_: Box<TirStmt>,
    },
    Return(Option<TirExpr>),
    Block(Vec<TirStmt>),
    Expr(TirExpr),
}
