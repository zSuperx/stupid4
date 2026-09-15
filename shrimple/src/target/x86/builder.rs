use smallvec::SmallVec;

use crate::{
    common::{BasicBlock, FunctionBuilder, InstructionTrait, Label, ModuleBuilder},
    target::{
        stir::{builder::IRFunction, isa::*},
        x86::isa::*,
    },
};
use std::collections::HashMap;

#[derive(Default)]
pub struct Metadata {
    pub(super) v2p: HashMap<IRValue, x86Value>,
    pub(super) v_rsp: i128,
    pub(super) ir_args: Vec<IRType>,
}

impl x86Function {
    pub fn lower(stir_function: &IRFunction) -> x86Function {
        // Create the function
        let rty = LLType::fromIRType(stir_function.getReturnType());
        let mut mcf = x86Function::new(stir_function.name.clone(), rty, Metadata::default());
        mcf.setRegCount(stir_function.getRegCount());

        // Handle ABI impl
        mcf.resolve_args(stir_function);

        // This creates the builder
        mcf.translate(stir_function);

        // Legalizes instructions that may have been mangled by conforming to the ABI
        mcf.legalize();

        if mcf.check_frame_emission() {
            mcf.emit_frame();
        }

        mcf.merge_degenerate_jumps();
        mcf
    }
}

pub type x86Label = Label<x86Instr>;
pub type x86BasicBlock = BasicBlock<x86Instr>;
pub type x86Function = FunctionBuilder<x86Instr, x86Value, LLType, Metadata>;
pub type x86Module = ModuleBuilder<x86Instr, x86Value, LLType, Metadata>;

impl x86Function {
    pub fn nextReg(&mut self, ty: LLType) -> Register {
        let ret = Register::Virt(self.reg_count, ty);
        self.reg_count += 1;
        ret
    }

    pub fn print(&self, include_comments: bool) {
        println!("{}:", self.name);
        self.dfs(|mcf, curr_id| {
            println!("{curr_id}:");
            let block = &mcf.blocks[&curr_id];
            for i in block.instructions.iter() {
                if matches!(i, x86Instr::Comment(..)) && !include_comments {
                    continue;
                }
                println!("\t{i}");
            }
            if block.terminator().is_none() {
                println!("\t; !! (missing terminator)");
            }
        });
    }
}

impl x86Instr {
    pub fn regUses(&self) -> SmallVec<[&Register; 4]> {
        let mut ret = SmallVec::new();
        for value in self.uses() {
            match value {
                x86Value::Reg(r) => ret.push(r),
                x86Value::Mem(AddressMode::Direct { base, index, .. }) => {
                    ret.push(base);
                    if let Some(index) = index {
                        ret.push(index);
                    }
                }
                _ => {}
            }
        }
        ret
    }

    pub fn regDefs(&self) -> SmallVec<[&Register; 4]> {
        let mut ret = SmallVec::new();
        for value in self.defs() {
            match value {
                x86Value::Reg(r) => ret.push(r),
                _ => {}
            }
        }
        ret
    }
}
