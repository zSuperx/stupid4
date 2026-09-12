use crate::target::x86::{Backend, isa::x86Instr};

use x86Instr::*;

impl Backend {
    /// Walks the function and merges blocks whose terminator is a direct jump to its fallthrough
    /// if the fallthrough target's only predecessor is the current block
    pub(crate) fn merge_degenerate_jumps(&mut self) {
        let mcf = self
            .builder
            .as_mut()
            .expect("Block merging can only happen after IR translation");

        mcf.dfs_mut(|mcf, curr_id| {
            // We loop each block to keep folding in children
            loop {
                let curr = mcf.blocks.get(&curr_id).unwrap();
                if let Some(Jmp(target_id)) = curr.terminator
                    && curr.fallthrough == Some(target_id)
                    && target_id != curr_id
                {
                    let target = mcf.blocks.get(&target_id).unwrap();
                    if target.predecessors.len() != 1 {
                        // If the target is the successor of multiple blocks, we can't merge
                        break;
                    }

                    // If the target and curr are atomic and sequential, remove target
                    let mut target = mcf.blocks.remove(&target_id).unwrap();

                    // curr inherits the rest of target's instructions, successors, fallthrough, and terminator
                    let curr = mcf.blocks.get_mut(&curr_id).unwrap();
                    curr.successors = target.successors;
                    curr.instructions
                        .push(Comment(format!("\r; BB: {target_id}")));
                    curr.instructions.append(&mut target.instructions);
                    curr.terminator = target.terminator;
                    curr.fallthrough = target.fallthrough;
                } else {
                    break;
                }
            }
        });
    }
}
