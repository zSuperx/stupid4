use crate::ast::*;
use crate::common::Spanned;

use super::*;

#[derive(Debug, Clone)]
pub enum HirStmt {
    Let {
        lhs: Spanned<&'static str>,
        ty: Option<Spanned<TypeId>>,
        rhs: Spanned<HirExpr>,
    },
    While {
        cond: Spanned<HirExpr>,
        body: Box<Spanned<HirStmt>>,
    },
    Continue,
    Break,
    If {
        cond: Spanned<HirExpr>,
        then_: Box<Spanned<HirStmt>>,
        else_: Box<Spanned<HirStmt>>,
    },
    Return(Option<Spanned<HirExpr>>),
    Block(Vec<Spanned<HirStmt>>),
    Expr(Spanned<HirExpr>),
}
