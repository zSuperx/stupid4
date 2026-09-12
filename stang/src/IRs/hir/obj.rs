use super::*;
use crate::ast::*;
use crate::common::Spanned;
use crate::common::RcString;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum HirObj {
    Fn(HirFunction),
    Global {
        name: Spanned<RcString>,
        ty: Spanned<Rc<RawType>>,
        rhs: Box<Spanned<HirExpr>>,
    },
    Struct {
        name: Spanned<RcString>,
        fields: Vec<(Spanned<RcString>, Spanned<Rc<RawType>>)>,
    },
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: Spanned<RcString>,
    pub return_type: Spanned<Rc<RawType>>,
    pub args: Vec<(Spanned<RcString>, Spanned<Rc<RawType>>)>,
    pub body: Spanned<HirStmt>,
}
