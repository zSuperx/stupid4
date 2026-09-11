use crate::target::stir::isa::IRType;

#[derive(Clone, Debug, Copy)]
pub enum LLType {
    I1,
    I8,
    I16,
    I32,
    I64,
}

impl LLType {
    pub fn fromIRType(ty: &IRType) -> LLType {
        match ty {
            IRType::I1 => LLType::I1,
            IRType::I8 => LLType::I8,
            IRType::I16 => LLType::I16,
            IRType::I32 => LLType::I32,
            IRType::Ptr | IRType::I64 => LLType::I64,
            IRType::Struct => todo!(),
        }
    }

    pub fn bytes(&self) -> usize {
        self.bits().div_ceil(8)
    }

    pub fn bits(&self) -> usize {
        match self {
            LLType::I1 => 8,
            LLType::I8 => 8,
            LLType::I16 => 16,
            LLType::I32 => 32,
            LLType::I64 => 64,
        }
    }

    pub fn width_str(&self) -> &'static str {
        match self {
            LLType::I1 => "FLAG",
            LLType::I8 => "byte ",
            LLType::I16 => "word ",
            LLType::I32 => "dword ",
            LLType::I64 => "qword ",
        }
    }
}
