use crate::common::RewriteAction;
use crate::target::x86::Backend;
use crate::target::x86::isa::*;

use x86Instr::*;

impl Backend {
    /// Walks over the instructions of a function and legalizes them based on operands.
    ///
    /// For example, `mov [%x], [%y]` will become `mov %tmp, [%y]` + `mov [%x], %tmp`
    pub(crate) fn legalize(&mut self) {
        let mcf = self
            .builder
            .as_mut()
            .expect("Legalization can only happen after IR translation");

        mcf.dfs_mut(|mcf, curr_id| {
            // TODO: This makes me feel like a borrow checker outlaw. Fix this shit
            let mut curr = mcf.blocks.remove(&curr_id).unwrap();
            curr.rewrite(|instr| match instr {
                Mov(mem1, mem2) if mem1.is_mem() && mem2.is_mem() => {
                    let tmp = x86Value::reg(mcf.nextReg(), mem1.ty());
                    RewriteAction::Replace(vec![Mov(tmp, *mem2), Mov(*mem1, tmp)])
                }
                _ => RewriteAction::Keep,
            });
            mcf.blocks.insert(curr_id, curr);
        })
    }
}
