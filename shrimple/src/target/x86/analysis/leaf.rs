use std::collections::HashSet;

use crate::target::x86::builder::*;
use crate::target::x86::isa::*;

impl x86Function {
    /// Searches for all blocks that terminate with a `Ret` or `Retv`.
    pub(crate) fn find_leaf_blocks(&self) -> HashSet<x86Label> {
        let mut exitpoints = HashSet::new();
        self.dfs(|self_, curr_id| {
            let curr = self_.blocks.get(&curr_id).unwrap();
            if let Some(x86Instr::Ret) = curr.terminator.as_ref() {
                exitpoints.insert(curr_id);
            };
        });
        exitpoints
    }
}
