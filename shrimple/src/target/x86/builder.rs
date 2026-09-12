use crate::{
    common::{BasicBlock, FunctionBuilder, Label},
    target::stir::builder::IRFunction,
    target::stir::isa::*,
    target::x86::isa::{LLType, Reg, x86Instr, x86Value},
};
use std::collections::HashMap;

#[derive(Default)]
pub struct x86Module {
    pub(super) v2p: HashMap<IRValue, x86Value>,
    pub(super) v_rsp: i128,
    pub(super) builder: Option<x86Function>,
    pub(super) ir_args: Vec<IRType>,
}

impl x86Module {
    pub fn lower(&mut self, stir_function: &mut IRFunction) -> x86Function {
        // Handle ABI impl
        self.resolve_args(stir_function);

        // This creates the builder
        self.translate(stir_function);

        // Legalizes instructions that may have been mangled by conforming to the ABI
        self.legalize();

        // Opt passes mutate the builder
        self.merge_degenerate_jumps();

        // Finally, take the builder out of self and return it
        self.builder.take().unwrap()
    }
}

pub type x86Label = Label<x86Instr>;
pub type x86BasicBlock = BasicBlock<x86Instr>;
pub type x86Function = FunctionBuilder<x86Instr, x86Value, LLType>;

impl x86Function {
    pub fn nextReg(&mut self) -> Reg {
        let ret = Reg::Virt(self.reg_count);
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
            if let Some(term) = &block.terminator {
                println!("\t{term}");
            } else {
                println!("\t; !! (missing terminator)");
            }
        });
    }
}
