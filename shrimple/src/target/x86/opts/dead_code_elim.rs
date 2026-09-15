use crate::target::x86::builder::*;
// use crate::target::x86::isa::*;
// use x86Instr::*;

impl x86Function {
    pub fn dead_code_elimination(&mut self) {
        let mut worklist: Vec<_> = self.find_leaf_blocks().into_iter().collect();

        while let Some(label) = worklist.pop() {
            let block = &self.blocks[&label];
        }
    }
}
