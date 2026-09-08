use crate::ast::*;
use crate::common::Spanned;

#[derive(Debug, Clone)]
pub enum HirExpr {
    Void,
    Num(&'static str),
    Bool(bool),
    Ident(&'static str),
    Assign {
        lhs: Box<Spanned<HirExpr>>,
        rhs: Box<Spanned<HirExpr>>,
    },
    AddrOf {
        inner: Box<Spanned<HirExpr>>,
    },
    SizeOfTy {
        ty: Spanned<TypeId>,
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
        field: &'static str,
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
        target_ty: Spanned<TypeId>,
        rhs: Box<Spanned<HirExpr>>,
    },
    Call {
        callee: Box<Spanned<HirExpr>>,
        args: Vec<Spanned<HirExpr>>,
    },
}
