use crate::{
    common::{BasicBlock, FunctionBuilder, Label, ModuleBuilder},
    stir::isa::{IRValue, VReg},
    target::stir::isa::{IRInstr, IRType},
};

pub type IRLabel = Label<IRInstr>;
pub type IRBasicBlock = BasicBlock<IRInstr>;
pub type IRFunction = FunctionBuilder<IRInstr, IRValue, IRType>;
pub type IRModule = ModuleBuilder<IRInstr, IRValue, IRType>;

impl IRFunction {
    pub fn nextReg(&mut self) -> VReg {
        let ret = self.reg_count;
        self.reg_count += 1;
        ret
    }

    pub fn print(&self, include_comments: bool) {
        println!(
            "{}({}):",
            self.name,
            self.args
                .iter()
                .map(|(name, ty)| format!("{name}: {ty}"))
                .collect::<Vec<String>>()
                .join(", ")
        );
        self.dfs(|mcf, curr_id| {
            println!("{curr_id}:");
            let block = &mcf.blocks[&curr_id];
            for i in block.instructions.iter() {
                if matches!(i, IRInstr::Comment(..)) && !include_comments {
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

    pub fn run_optimizations(&mut self) {
        self.liveness_analysis();
    }
}
