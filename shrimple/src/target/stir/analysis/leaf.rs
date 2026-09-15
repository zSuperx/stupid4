use std::collections::HashSet;

use crate::stir::{
    builder::{IRFunction, IRLabel},
    isa::IRInstr,
};

impl IRFunction {
    /// Searches for all blocks that terminate with a `Ret` or `Retv`.
    pub(crate) fn find_leaf_blocks(&self) -> HashSet<IRLabel> {
        let mut exitpoints = HashSet::new();
        self.dfs(|self_, curr_id| {
            let curr = self_.blocks.get(&curr_id).unwrap();
            if let Some(IRInstr::Ret(..) | IRInstr::Retv) = curr.terminator() {
                exitpoints.insert(curr_id);
            };
        });
        exitpoints
    }
}
