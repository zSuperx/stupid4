use smallvec::SmallVec;

use crate::{
    common::{BasicBlock, FunctionBuilder, InstructionTrait, Label, ModuleBuilder},
    target::{
        stir::{builder::IRFunction, isa::*},
        x86::isa::*,
    },
};
use std::{collections::HashMap, io::Write};

#[derive(Default)]
pub struct Metadata {
    pub(super) v2p: HashMap<IRValue, x86Value>,
    pub(super) v_rsp: i128,
    pub(super) frame_slots: Vec<LLType>,
    pub(super) ir_args: Vec<IRType>,
}

impl x86Function {
    pub fn lower(stir_function: &IRFunction) -> x86Function {
        // Create the function
        let rty = LLType::fromIRType(stir_function.getReturnType());
        let mut mcf = x86Function::new(stir_function.name.clone(), rty, Metadata::default());

        // Handle ABI impl
        mcf.resolve_args(stir_function);

        // This creates the builder
        mcf.translate(stir_function);

        // Legalizes instructions that may have been mangled by conforming to the ABI
        mcf.legalize();

        mcf.dead_code_elimination();

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
        let ret = Virt(self.reg_count, ty);
        self.reg_count += 1;
        ret
    }

    pub fn createRegValue(&mut self, ty: LLType) -> x86Value {
        let reg = self.nextReg(ty);
        Reg(reg)
    }

    pub fn createMemValue(&mut self, ty: LLType) -> x86Value {
        let base = self.nextReg(LLType::I64);
        Mem(AddressMode::Direct {
            base,
            index: None,
            scale: 0,
            disp: 0,
            ty,
        })
    }

    pub fn print(&self, writer: &mut Box<dyn Write>, include_comments: bool) {
        writeln!(writer, "{}:", self.name);
        self.dfs(|mcf, curr_id| {
            writeln!(writer, "{curr_id}:");
            let block = &mcf.blocks[&curr_id];
            for i in block.instructions.iter() {
                if matches!(i, x86Instr::Comment(..)) && !include_comments {
                    continue;
                }
                writeln!(writer, "\t{i}");
            }
            if block.terminator().is_none() {
                writeln!(writer, "\t; !! (missing terminator)");
            }
        });
        writeln!(writer);
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
