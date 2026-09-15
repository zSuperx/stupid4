use std::io::Write;

use crate::{
    common::{BasicBlock, FunctionBuilder, Label, ModuleBuilder},
    stir::isa::{IRValue, VReg},
    target::stir::isa::{IRInstr, IRType},
};

pub type IRLabel = Label<IRInstr>;
pub type IRBasicBlock = BasicBlock<IRInstr>;
pub type IRFunction = FunctionBuilder<IRInstr, IRValue, IRType, ()>;
pub type IRModule = ModuleBuilder<IRInstr, IRValue, IRType, ()>;

impl IRFunction {
    pub fn createVReg(&mut self, ty: IRType) -> IRValue {
        let ret = self.reg_count;
        self.reg_count += 1;
        if ty.is_pointer() {
            IRValue::Ptr(ret, ty)
        } else {
            IRValue::Reg(ret, ty)
        }
    }

    pub fn print(&self, mut writer: &mut Box<dyn Write>, include_comments: bool) {
        writeln!(
            writer,
            "{}({}):",
            self.symbol,
            self.args
                .iter()
                .map(|name| format!("{name}"))
                .collect::<Vec<String>>()
                .join(", ")
        );
        self.dfs(|mcf, curr_id| {
            writeln!(writer, "{curr_id}:");
            let block = &mcf.blocks[&curr_id];
            for i in block.instructions.iter() {
                if matches!(i, IRInstr::Comment(..)) && !include_comments {
                    continue;
                }
                writeln!(writer, "\t{i}");
            }
            if block.terminator().is_none() {
                writeln!(writer, "\t; !! (missing terminator)");
            }
        });
    }

    pub fn run_optimizations(&mut self) {
        self.liveness_analysis();
    }
}
