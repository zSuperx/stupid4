use crate::{
    common::{BasicBlock, FunctionBuilder, Label, ModuleBuilder},
    target::{
        stir::{builder::IRFunction, isa::*},
        x86::isa::{LLType, Reg, x86Instr, x86Value},
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
        let mut new_function =
            x86Function::new(stir_function.name.clone(), rty, Metadata::default());
        new_function.setRegCount(stir_function.getRegCount());

        // Handle ABI impl
        new_function.resolve_args(stir_function);

        // This creates the builder
        new_function.translate(stir_function);

        // Legalizes instructions that may have been mangled by conforming to the ABI
        new_function.legalize();

        // Opt passes mutate the builder
        new_function.merge_degenerate_jumps();
        new_function
    }
}

pub type x86Label = Label<x86Instr>;
pub type x86BasicBlock = BasicBlock<x86Instr>;
pub type x86Function = FunctionBuilder<x86Instr, x86Value, LLType, Metadata>;
pub type x86Module = ModuleBuilder<x86Instr, x86Value, LLType, Metadata>;

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
