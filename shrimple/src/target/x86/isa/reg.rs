#[repr(usize)]
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub enum Register {
    AL = 0,
    AH = 1,
    AX = 2,
    EAX = 3,
    RAX = 4,

    BL = 5,
    BH = 6,
    BX = 7,
    EBX = 8,
    RBX = 9,

    CL = 10,
    CH = 11,
    CX = 12,
    ECX = 13,
    RCX = 14,

    DL = 15,
    DH = 16,
    DX = 17,
    EDX = 18,
    RDX = 19,

    SIL = 20,
    SI = 21,
    ESI = 22,
    RSI = 23,

    DIL = 24,
    DI = 25,
    EDI = 26,
    RDI = 27,

    SPL = 28,
    SP = 29,
    ESP = 30,
    RSP = 31,

    BPL = 32,
    BP = 33,
    EBP = 34,
    RBP = 35,

    R8B = 36,
    R8W = 37,
    R8D = 38,
    R8 = 39,

    R9B = 40,
    R9W = 41,
    R9D = 42,
    R9 = 43,

    R10B = 44,
    R10W = 45,
    R10D = 46,
    R10 = 47,

    R11B = 48,
    R11W = 49,
    R11D = 50,
    R11 = 51,

    R12B = 52,
    R12W = 53,
    R12D = 54,
    R12 = 55,

    R13B = 56,
    R13W = 57,
    R13D = 58,
    R13 = 59,

    R14B = 60,
    R14W = 61,
    R14D = 62,
    R14 = 63,

    R15B = 64,
    R15W = 65,
    R15D = 66,
    R15 = 67,

    Virt(usize, LLType),
}

pub use Register::*;

use crate::target::x86::isa::LLType;

impl Register {
    pub fn get_type(&self) -> LLType {
        match self {
            Virt(_, lltype) => *lltype,
            _ => todo!(),
        }
    }
}

impl From<&Register> for usize {
    fn from(other: &Register) -> usize {
        match other {
            Virt(x, _) => x + 68,
            x => usize::from(x),
        }
    }
}

impl From<Register> for usize {
    fn from(other: Register) -> usize {
        match other {
            Virt(x, _) => x + 68,
            x => usize::from(x),
        }
    }
}

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Virt(v, ty) => {
                let size_char = match ty {
                    LLType::I8 => "b",
                    LLType::I16 => "w",
                    LLType::I32 => "d",
                    LLType::I64 => "q",
                };
                f.write_fmt(format_args!("%{v}{size_char}"))
            }
            _ => format!("{self:?}").to_lowercase().fmt(f),
        }
    }
}
