use super::*;
use crate::ast::*;
use crate::common::Spanned;

#[derive(Debug, Clone)]
pub enum HirObj {
    Fn(HirFunction),
    Global {
        name: Spanned<&'static str>,
        ty: Spanned<TypeId>,
        rhs: Box<Spanned<HirExpr>>,
    },
    Struct {
        name: Spanned<&'static str>,
        fields: Vec<(Spanned<&'static str>, Spanned<TypeId>)>,
    },
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: Spanned<&'static str>,
    pub return_type: Spanned<TypeId>,
    pub args: Vec<(Spanned<&'static str>, Spanned<TypeId>)>,
    pub body: Spanned<HirStmt>,
}
