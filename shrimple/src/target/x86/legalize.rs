use crate::common::RewriteAction;
use crate::target::x86::isa::*;
use crate::x86Function;

use x86Instr::*;

impl x86Function {
    /// Walks over the instructions of a function and legalizes them based on operands.
    ///
    /// For example, `mov [%x], [%y]` will become `mov %tmp, [%y]` + `mov [%x], %tmp`
    pub(crate) fn legalize(&mut self) {
        self.dfs_mut(|self_, curr_id| {
            // TODO: This makes me feel like a borrow checker outlaw. Fix this shit
            let mut curr = self_.blocks.remove(&curr_id).unwrap();
            curr.rewrite(|instr| match instr {
                Mov(mem1, mem2) if mem1.is_mem() && mem2.is_mem() => {
                    assert_eq!(mem1.get_type(), mem2.get_type(), "Memory operands are of different sizes");
                    let tmp = Reg(self_.nextReg(mem1.get_type()));
                    RewriteAction::Replace(vec![Mov(tmp, *mem2), Mov(*mem1, tmp)])
                }
                _ => RewriteAction::Keep,
            });
            self_.blocks.insert(curr_id, curr);
        })
    }
}
