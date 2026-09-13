use std::hash::Hash;
use std::rc::Rc;

use crate::{common::RcString, die, translation_unit::qtype};
use shrimple::stir::isa::IRType;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum QualType {
    // Primitive types
    I8,
    I16,
    I32,
    I64,

    U8,
    U16,
    U32,
    U64,

    Bool,
    Void,

    // Rest
    Struct {
        name: RcString,
        fields: Vec<(RcString, Rc<QualType>)>,
    },
    Function {
        arg_types: Vec<Rc<QualType>>,
        return_type: Rc<QualType>,
    },
    Pointer(Rc<QualType>),
}

impl QualType {
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            QualType::I8
                | QualType::U8
                | QualType::I16
                | QualType::U16
                | QualType::I32
                | QualType::U32
                | QualType::I64
                | QualType::U64
        )
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            QualType::I8
                | QualType::U8
                | QualType::I16
                | QualType::U16
                | QualType::I32
                | QualType::U32
                | QualType::I64
                | QualType::U64
                | QualType::Bool
        )
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            QualType::I8 | QualType::I16 | QualType::I32 | QualType::I64
        )
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, QualType::Pointer(..) | QualType::Struct { .. })
    }

    pub fn get_pointee(&self) -> Rc<QualType> {
        match self {
            QualType::Pointer(p) => p.clone(),
            _ => die!("Not a pointer type: {self}"),
        }
    }

    pub fn create_pointer(&self) -> Rc<QualType> {
        match self {
            QualType::Function { .. } => qtype(self),
            _ => qtype(&QualType::Pointer(qtype(self))),
        }
    }

    pub fn alignment(&self) -> usize {
        match self {
            QualType::Struct { .. } => todo!(),
            // Function types are coerced to pointers
            x => x.bits(),
        }
    }

    pub fn bits(&self) -> usize {
        match self {
            QualType::I8 | QualType::U8 => 8,
            QualType::I16 | QualType::U16 => 16,
            QualType::I32 | QualType::U32 => 32,
            QualType::I64 | QualType::U64 => 64,
            QualType::Bool => 1,
            QualType::Void => 0,
            QualType::Function { .. } => 64,
            QualType::Pointer(_) => 64,
            QualType::Struct { .. } => todo!(),
        }
    }

    pub fn bytes(&self) -> usize {
        self.bits().div_ceil(8)
    }

    pub fn toIRType(&self) -> IRType {
        match self {
            QualType::I8 | QualType::U8 => IRType::I8,
            QualType::I16 | QualType::U16 => IRType::I16,
            QualType::I32 | QualType::U32 | QualType::Void => IRType::I32,
            QualType::I64 | QualType::U64 => IRType::I64,
            QualType::Bool => IRType::I8,
            QualType::Pointer(..) => IRType::Ptr,
            QualType::Struct { .. } => IRType::Ptr,
            QualType::Function { .. } => IRType::Ptr,
            _ => panic!("Can't lower {self:?} type"),
        }
    }
}

impl std::fmt::Display for QualType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualType::Struct { name, .. } => name.fmt(f),
            QualType::Function {
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
            QualType::Pointer(type_id) => f.write_fmt(format_args!("*{}", *type_id)),
            x => f.write_fmt(format_args!("{}", format!("{x:?}").to_lowercase())),
        }
    }
}
