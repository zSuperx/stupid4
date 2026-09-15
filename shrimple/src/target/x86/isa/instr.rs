#![allow(unused)]
#![allow(nonstandard_style)]
mod generated {
    include!("./generated.rs");
}

use crate::{common::InstructionTrait, target::x86::isa::*};
pub use generated::*;
use x86Instr::*;

impl x86Instr {
    pub fn has_side_effects(&self) -> bool {
        match self {
            Comment(..) | Ret | Call(..) | Push(..) => true,
            _ => {
                for def in self.defs() {
                    if def.is_mem() && !matches!(def, Mem(AddressMode::FrameSlot { .. })) {
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut <Self as InstructionTrait>::Val> {
        match self {
            Add(v1, v2) => [v1, v2].into_iter(),
            _ => todo!(),
        }
    }
}
