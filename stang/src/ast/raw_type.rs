use std::hash::Hash;
use std::rc::Rc;

use crate::{common::RcString, die, translation_unit::qtype};
use shrimple::stir::isa::IRType;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum RawType {
    Base(RcString),
    Pointer(Rc<RawType>),
    Function {
        arg_types: Vec<Rc<RawType>>,
        return_type: Rc<RawType>,
    },
}

impl std::fmt::Display for RawType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawType::Pointer(type_id) => f.write_fmt(format_args!("*{}", *type_id)),
            RawType::Function {
                arg_types,
                return_type,
            } => {
                f.write_str("fn(")?;
                let mut iter = arg_types.iter().peekable();
                while let Some(ty) = iter.next() {
                    ty.fmt(f)?;
                    if iter.peek().is_some() {
                        f.write_str(", ")?;
                    }
                }
                f.write_fmt(format_args!("):{return_type}"))
            }
            RawType::Base(s) => s.fmt(f),
        }
    }
}
