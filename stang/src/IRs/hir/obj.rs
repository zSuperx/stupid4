use super::*;
use crate::ast::*;
use crate::common::RcString;
use crate::common::Spanned;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct HirGlobal {
    pub name: Spanned<RcString>,
    pub ty: Spanned<Rc<RawType>>,
    pub rhs: Box<Spanned<HirExpr>>,
}

#[derive(Debug, Clone)]
pub struct HirStruct {
    pub name: Spanned<RcString>,
    pub fields: Vec<(Spanned<RcString>, Spanned<Rc<RawType>>)>,
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: Spanned<RcString>,
    pub return_type: Spanned<Rc<RawType>>,
    pub args: Vec<(Spanned<RcString>, Spanned<Rc<RawType>>)>,
    pub body: Spanned<HirStmt>,
}
