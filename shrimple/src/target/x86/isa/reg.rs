use crate::target::x86::isa::{types::LLType, value::x86Value};

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Reg {
    A,
    B,
    C,
    D,
    SI,
    DI,
    SP,
    BP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Virt(usize),
}

impl From<usize> for Reg {
    fn from(value: usize) -> Self {
        use Reg::*;
        match value {
            0 => A,
            1 => B,
            2 => C,
            3 => D,
            4 => SI,
            5 => DI,
            6 => SP,
            7 => BP,
            8 => R8,
            9 => R9,
            10 => R10,
            11 => R11,
            12 => R12,
            13 => R13,
            14 => R14,
            15 => R15,
            x => Virt(x),
        }
    }
}

impl From<Reg> for usize {
    fn from(other: Reg) -> usize {
        use Reg::*;
        match other {
            A => 0,
            B => 1,
            C => 2,
            D => 3,
            SI => 4,
            DI => 5,
            SP => 6,
            BP => 7,
            R8 => 8,
            R9 => 9,
            R10 => 10,
            R11 => 11,
            R12 => 12,
            R13 => 13,
            R14 => 14,
            R15 => 15,
            Virt(x) => x + 16,
        }
    }
}

impl Reg {
    pub fn sized_print(&self, f: &mut std::fmt::Formatter<'_>, bits: usize) -> std::fmt::Result {
        let names = match self {
            Reg::A => ["al", "ax", "eax", "rax"],
            Reg::B => ["bl", "bx", "ebx", "rbx"],
            Reg::C => ["cl", "cx", "ecx", "rcx"],
            Reg::D => ["dl", "dx", "edx", "rdx"],
            Reg::SI => ["sil", "si", "esi", "rsi"],
            Reg::DI => ["dil", "di", "edi", "rdi"],
            Reg::SP => ["spl", "sp", "esp", "rsp"],
            Reg::BP => ["bpl", "bp", "ebp", "rbp"],
            Reg::R8 => ["r8b", "r8w", "r8d", "r8"],
            Reg::R9 => ["r9b", "r9w", "r9d", "r9"],
            Reg::R10 => ["r10b", "r10w", "r10d", "r10"],
            Reg::R11 => ["r11b", "r11w", "r11d", "r11"],
            Reg::R12 => ["r12b", "r12w", "r12d", "r12"],
            Reg::R13 => ["r13b", "r13w", "r13d", "r13"],
            Reg::R14 => ["r14b", "r14w", "r14d", "r14"],
            Reg::R15 => ["r15b", "r15w", "r15d", "r15"],
            Reg::Virt(v) => {
                let width_spec = match bits {
                    8 => "b",
                    16 => "w",
                    32 => "d",
                    64 => "q",
                    _ => panic!("Size: {bits} not supported"),
                };
                return f.write_fmt(format_args!("%{v}{width_spec}"));
            }
        };
        let sized_name = match bits {
            8 => names[0],
            16 => names[1],
            32 => names[2],
            64 => names[3],
            _ => {
                panic!("Size: {bits} not supported")
            }
        };
        f.write_str(sized_name)
    }
}

pub const RBP: x86Value = x86Value::reg(Reg::BP, LLType::I64);
pub const RSP: x86Value = x86Value::reg(Reg::SP, LLType::I64);
pub const RAX: x86Value = x86Value::reg(Reg::A, LLType::I64);
pub const EAX: x86Value = x86Value::reg(Reg::A, LLType::I32);

pub const RDI: x86Value = x86Value::reg(Reg::DI, LLType::I64);
pub const RSI: x86Value = x86Value::reg(Reg::SI, LLType::I64);
pub const RDX: x86Value = x86Value::reg(Reg::D, LLType::I64);
pub const RCX: x86Value = x86Value::reg(Reg::C, LLType::I64);
pub const R8Q: x86Value = x86Value::reg(Reg::R8, LLType::I64);
pub const R9Q: x86Value = x86Value::reg(Reg::R9, LLType::I64);
