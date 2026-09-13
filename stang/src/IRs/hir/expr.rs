use crate::ast::*;
use crate::common::{RcString, Spanned};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum HirExpr {
    Void,
    Num(RcString),
    Bool(bool),
    Ident(RcString),
    Assign {
        lhs: Box<Spanned<HirExpr>>,
        rhs: Box<Spanned<HirExpr>>,
    },
    AddrOf {
        inner: Box<Spanned<HirExpr>>,
    },
    SizeOfTy {
        ty: Spanned<Rc<RawType>>,
    },
    SizeOfExpr {
        expr: Box<Spanned<HirExpr>>,
    },
    Deref {
        inner: Box<Spanned<HirExpr>>,
    },
    Index {
        base: Box<Spanned<HirExpr>>,
        index: Box<Spanned<HirExpr>>,
    },
    Field {
        base: Box<Spanned<HirExpr>>,
        field: RcString,
    },
    Un {
        op: UnOp,
        rhs: Box<Spanned<HirExpr>>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Spanned<HirExpr>>,
        rhs: Box<Spanned<HirExpr>>,
    },
    Cast {
        target_ty: Spanned<Rc<RawType>>,
        rhs: Box<Spanned<HirExpr>>,
    },
    Call {
        callee: Box<Spanned<HirExpr>>,
        args: Vec<Spanned<HirExpr>>,
    },
}
