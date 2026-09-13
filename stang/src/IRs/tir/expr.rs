use crate::ast::*;
use crate::common::*;
use crate::translation_unit::Symbol;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct TirExpr {
    pub kind: TirExprKind,
    pub ty: Rc<QualType>,
}

impl TirExpr {
    pub fn new(kind: TirExprKind, ty: Rc<QualType>) -> Self {
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
        target_ty: Rc<QualType>,
        expr: Box<TirExpr>,
    },
    Call {
        callee: Box<TirExpr>,
        args: Vec<TirExpr>,
    },
}
