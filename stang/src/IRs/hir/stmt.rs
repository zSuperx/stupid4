use crate::ast::*;
use crate::common::Spanned;
use crate::common::RcString;
use std::rc::Rc;

use super::*;

#[derive(Debug, Clone)]
pub enum HirStmt {
    LetDecl {
        name: Spanned<RcString>,
        ty: Option<Spanned<Rc<RawType>>>,
    },
    LetAssign {
        name: Spanned<RcString>,
        ty: Option<Spanned<Rc<RawType>>>,
        value: Spanned<HirExpr>,
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
