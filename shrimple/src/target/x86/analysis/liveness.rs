use std::collections::BTreeMap;

use bitset::BitSet;

use crate::{common::InstructionTrait, target::x86::builder::{x86Label, x86Function}};

impl x86Function {
    /// Performs liveness analysis and returns a ???
    pub(crate) fn liveness_analysis(&mut self) {
        let mut exitpoints = self.find_leaf_blocks();
        let mut worklist = vec![];

        let mut LIVE_IN = BTreeMap::<x86Label, BitSet>::new();
        let mut LIVE_OUT = BTreeMap::<x86Label, BitSet>::new();

        // Insert empty sets for every reachable block
        self.dfs(|self_, curr_id| {
            worklist.push(curr_id);
            LIVE_IN.insert(curr_id, Default::default());
            LIVE_OUT.insert(curr_id, Default::default());
        });

        while let Some(curr_id) = worklist.pop() {
            let curr = self.blocks.get_mut(&curr_id).unwrap();

            // LIVE_IN[s] = GEN[s] U (LIVE_OUT[s] - KILL[s])
            let mut live_in = LIVE_OUT[&curr_id].clone();
            for i in curr.instructions.iter().rev() {
                // Iterate through all defs of an instruction
                // If we find a def, it is not LIVE_IN
                for d in i.defs() {
                    for reg in d.getReg() {
                        live_in.remove(reg.into());
                    }
                }

                // Iterate through all uses of an instruction
                for u in i.uses() {
                    for reg in u.getReg() {
                        live_in.insert(reg.into());
                    }
                }
            }

            // LIVE_OUT[s] = U LIVE_IN[succ] forall succ of s
            for succ_id in curr.successors.iter() {
                LIVE_OUT
                    .get_mut(&curr_id)
                    .unwrap()
                    .union_eq(&LIVE_IN[&succ_id]);
            }

            // If LIVE_IN[s] changed, predecessors need to be recomputed since their LIVE_OUTs
            // derive from LIVE_IN[s]
            let live_in_changed = live_in != LIVE_IN[&curr_id];
            LIVE_IN.insert(curr_id, live_in);
            if live_in_changed {
                for pred_id in curr.predecessors.iter() {
                    worklist.push(*pred_id);
                }
            }
        }

        self.dfs(|self_, curr_id| {
            let curr = self_.blocks.get_mut(&curr_id).unwrap();
            curr.live_in = std::mem::take(LIVE_IN.get_mut(&curr_id).unwrap());
            curr.live_out = std::mem::take(LIVE_OUT.get_mut(&curr_id).unwrap());
        });
    }
}
