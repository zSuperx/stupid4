use smallvec::{SmallVec, smallvec};

use crate::common::ModuleSymbol;

use super::IRType;

pub type VReg = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRValue {
    Reg(VReg),
    Ptr(VReg),
    Imm(i128),
    Sym(ModuleSymbol),
}

impl IRValue {
    /// If the provided `ty` is a pointer, wraps `reg` in `IRValue::Ptr`, else `IRValue::Reg`
    pub fn typed(reg: VReg, ty: IRType) -> Self {
        if ty.is_pointer() {
            IRValue::Ptr(reg)
        } else {
            IRValue::Reg(reg)
        }
    }

    /// Returns any underlying registers used in this value.
    ///
    /// A `SmallVec` is returned to reserve the API for IRValues that may contain more than 1
    /// register in the future, as well as being consistent with target-specific APIs.
    pub fn getReg(&self) -> SmallVec<[VReg; 2]> {
        match self {
            IRValue::Reg(r) | IRValue::Ptr(r) => smallvec![*r],
            _ => smallvec![],
        }
    }

    /// Rewrites all instances of `old` with `new` within the value.
    pub fn rewriteReg(&mut self, old: VReg, new: VReg) {
        match self {
            IRValue::Reg(r) | IRValue::Ptr(r) if old == *r => {
                *r = new;
            }
            _ => {}
        }
    }

    pub fn is_mem(&self) -> bool {
        matches!(self, IRValue::Ptr(..))
    }

    pub fn is_reg(&self) -> bool {
        matches!(self, IRValue::Reg(..))
    }

    pub fn is_imm(&self) -> bool {
        matches!(self, IRValue::Imm(..))
    }
}

impl std::fmt::Display for IRValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRValue::Reg(r) => f.write_fmt(format_args!("%{r}")),
            IRValue::Imm(i) => f.write_fmt(format_args!("#{i}")),
            IRValue::Ptr(r) => f.write_fmt(format_args!("%{r}")),
            IRValue::Sym(s) => f.write_fmt(format_args!("@{s}")),
        }
    }
}
