use crate::ast::*;
use crate::common::*;

#[derive(Debug, Clone)]
pub struct TirExpr {
    pub kind: TirExprKind,
    pub ty: TypeId,
}

impl TirExpr {
    pub fn new(kind: TirExprKind, ty: TypeId) -> Self {
        Self { kind, ty }
    }
}

#[derive(Debug, Clone)]
pub enum TirExprKind {
    Num(i128),
    Bool(bool),
    Store {
        ptr: Box<TirExpr>,
        val: Box<TirExpr>,
    },
    Load {
        inner: Box<TirExpr>,
    },
    ValueOf(Symbol),
    AddrOf(Symbol),
    Un {
        op: UnOp,
        rhs: Box<TirExpr>,
    },
    Bin {
        op: BinOp,
        lhs: Box<TirExpr>,
        rhs: Box<TirExpr>,
    },
    Cast {
        target_ty: TypeId,
        expr: Box<TirExpr>,
    },
    IndirectCall {
        callee: Box<TirExpr>,
        args: Vec<TirExpr>,
    },
    DirectCall {
        callee: Symbol,
        args: Vec<TirExpr>,
    },
}
