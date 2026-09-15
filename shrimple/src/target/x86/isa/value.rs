use std::fmt::Display;

use smallvec::{SmallVec, smallvec};

use crate::common::ModuleSymbol;

use super::reg::*;
use super::types::*;

#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub enum RFLAG {
    LT,
    LE,
    GT,
    GE,
    EQ,
    NE,

    /// Zero
    Z,
    /// Not zero
    NZ,

    /// Overflow
    O,
    /// No overflow
    NO,
}

impl Display for RFLAG {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{self:?}").to_lowercase();
        f.write_str(s.as_str())
    }
}

#[derive(Clone, Debug, Copy)]
pub enum AddressMode {
    // [base + (index * scale) + disp]
    Direct {
        base: Register,
        index: Option<Register>,
        scale: usize,
        disp: i128,
        /// The low-level type this pointer points to
        ty: LLType,
    },
    // [rel symbol]
    Relative(ModuleSymbol),
}

#[derive(Clone, Debug, Copy)]
pub enum x86Value {
    Imm(i128),
    Reg(Register),
    Mem(AddressMode),
    CC(RFLAG),
    Sym(ModuleSymbol),
}

#[allow(unused_imports)]
pub use x86Value::{CC, Imm, Mem, Reg, Sym};

impl x86Value {
    pub fn get_type(&self) -> LLType {
        match self {
            Reg(register) => register.get_type(),
            Mem(AddressMode::Direct { ty, .. }) => *ty,
            _ => panic!("No type for {self}"),
        }
    }

    pub fn is_mem(&self) -> bool {
        matches!(self, x86Value::Mem { .. })
    }

    pub fn is_reg(&self) -> bool {
        matches!(self, x86Value::Reg { .. })
    }

    pub const fn mem(base: Register, ty: LLType) -> x86Value {
        x86Value::Mem(AddressMode::Direct {
            base,
            index: None,
            scale: 1,
            disp: 0,
            ty,
        })
    }

    pub fn getReg(&self) -> SmallVec<[Register; 2]> {
        match self {
            x86Value::Reg(name) => smallvec![*name],
            _ => smallvec![],
        }
    }

    pub fn rewriteReg(&mut self, old: Register, new: Register) {
        match self {
            x86Value::Reg(name) if old == *name => {
                *name = new;
            }
            x86Value::Mem(AddressMode::Direct { base, index, .. }) => {
                if *base == old {
                    *base = new;
                }
                if let Some(inner) = index
                    && *inner == old
                {
                    *inner = new;
                }
            }
            _ => {}
        }
    }

    pub const fn memDisp(base: Register, disp: i128, ty: LLType) -> x86Value {
        x86Value::Mem(AddressMode::Direct {
            base,
            index: None,
            scale: 1,
            disp,
            ty,
        })
    }

    pub const fn memDirect(
        base: Register,
        index: Option<Register>,
        scale: usize,
        disp: i128,
        ty: LLType,
    ) -> x86Value {
        x86Value::Mem(AddressMode::Direct {
            base,
            index,
            scale,
            disp,
            ty,
        })
    }
}

impl Display for x86Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            x86Value::Imm(i) => i.fmt(f),
            x86Value::Sym(s) => s.fmt(f),
            x86Value::Reg(name) => name.fmt(f),
            x86Value::Mem(AddressMode::Direct {
                base,
                index,
                scale,
                disp,
                ty,
            }) => {
                f.write_str(ty.width_str())?;
                f.write_str("[")?;
                base.fmt(f)?;
                if let Some(i) = index {
                    assert_ne!(*i, Register::SP);
                    f.write_str(" + ")?;
                    i.fmt(f)?;
                    if *scale > 1 {
                        f.write_fmt(format_args!("*{scale}"))?;
                    }
                }

                match disp {
                    ..0 => f.write_fmt(format_args!(" - {}", disp.abs()))?,
                    0 => {}
                    1.. => f.write_fmt(format_args!(" + {}", disp.abs()))?,
                }
                f.write_str("]")
            }
            x86Value::Mem(AddressMode::Relative(symbol)) => {
                f.write_fmt(format_args!("[rel {symbol}]"))
            }
            x86Value::CC(flags) => flags.fmt(f),
        }
    }
}
