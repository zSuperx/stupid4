use std::collections::HashSet;

use crate::{
    common::{InstructionTrait, RewriteAction},
    target::x86::builder::*,
};
// use crate::target::x86::isa::*;
// use x86Instr::*;

impl x86Function {
    pub fn dead_code_elimination(&mut self) {
        let mut worklist: Vec<_> = self.find_leaf_blocks().into_iter().collect();

        let mut important_values = HashSet::new();

        while let Some(label) = worklist.pop() {
            let block = &self.blocks[&label];

            let mut changed = false;

            for instr in block.instructions.iter().rev() {
                if instr.has_side_effects() {
                    for use_ in instr.uses() {
                        changed |= important_values.insert(*use_);
                    }
                }

                for def in instr.defs() {
                    if important_values.contains(def) {
                        for use_ in instr.uses() {
                            changed |= important_values.insert(*use_);
                        }
                    }
                }
            }

            if changed {
                worklist.extend(block.predecessors.iter());
            }
        }

        self.dfs_mut(|self_, label| {
            let block = self_.blocks.get_mut(&label).unwrap();
            block.rewrite(|instr| {
                if instr.has_side_effects() || instr.is_terminator() {
                    return RewriteAction::Keep;
                }

                for def in instr.defs() {
                    if important_values.contains(def) {
                        return RewriteAction::Keep;
                    }
                }
                RewriteAction::Skip
            })
        });
    }
}
