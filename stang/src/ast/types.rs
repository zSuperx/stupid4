use std::hash::Hash;

use crate::{die, translation_unit::add_type};
use registry::*;
use shrimple::stir::isa::IRType;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum Type {
    Unresolved(&'static str),

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
    Base {
        name: &'static str,
        fields: Vec<(&'static str, TypeId)>,
    },
    Function {
        arg_types: Vec<TypeId>,
        return_type: TypeId,
    },
    Pointer(TypeId),
}

impl Type {
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            Type::I8
                | Type::U8
                | Type::I16
                | Type::U16
                | Type::I32
                | Type::U32
                | Type::I64
                | Type::U64
        )
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Type::I8
                | Type::U8
                | Type::I16
                | Type::U16
                | Type::I32
                | Type::U32
                | Type::I64
                | Type::U64
                | Type::Bool
        )
    }

    pub fn is_signed(&self) -> bool {
        matches!(self, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(..) | Type::Base { .. })
    }

    pub fn get_pointee(&self) -> TypeId {
        match self {
            Type::Pointer(f) if let Type::Function { .. } = f.lookup() => add_type(self.clone()),
            Type::Pointer(p) => *p,
            _ => die!("Not a pointer type: {self}"),
        }
    }

    pub fn alignment(&self) -> usize {
        match self {
            Type::Base { .. } => todo!(),
            // Function types are coerced to pointers
            x => x.bits(),
        }
    }

    pub fn bits(&self) -> usize {
        match self {
            Type::I8 | Type::U8 => 8,
            Type::I16 | Type::U16 => 16,
            Type::I32 | Type::U32 => 32,
            Type::I64 | Type::U64 => 64,
            Type::Bool => 1,
            Type::Void => 0,
            Type::Function { .. } => 64,
            Type::Pointer(_) => 64,
            Type::Base { .. } => todo!(),
            Self::Unresolved(..) => panic!(),
        }
    }

    pub fn bytes(&self) -> usize {
        self.bits().div_ceil(8)
    }

    pub fn toIRType(&self) -> IRType {
        match self {
            Type::I8 | Type::U8 => IRType::I8,
            Type::I16 | Type::U16 => IRType::I16,
            Type::I32 | Type::U32 | Type::Void => IRType::I32,
            Type::I64 | Type::U64 => IRType::I64,
            Type::Bool => IRType::I8,
            Type::Pointer(..) => IRType::Ptr,
            Type::Base { .. } => IRType::Ptr,
            Type::Function { .. } => IRType::Ptr,
            _ => panic!("Can't lower {self:?} type"),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Type::Base { name, .. } => format_args!("{}", *name),
            Type::Function {
                arg_types: args,
                return_type: returns,
            } => {
                format_args!(
                    "Fn({}) -> {}",
                    args.iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    *returns
                )
            }
            Type::Pointer(type_id) => format_args!("*{}", *type_id),
            x => format_args!("{}", format!("{x:?}").to_lowercase()),
        };
        f.write_fmt(s)
    }
}

pub type TypeId = Id<Type>;
