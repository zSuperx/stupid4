use crate::target::x86::builder::*;
use crate::target::x86::isa::*;
use x86Instr::*;

impl x86Function {
    /// Returns whether or not frame setup/teardown needs to occur.
    ///
    /// This is essentially just checking if any instruction writes to the stack pointer (SP)
    pub fn check_frame_emission(&self) -> bool {
        let mut requires_frame = false;
        self.dfs(|self_, curr_id| {
            let curr = self_.blocks.get(&curr_id).unwrap();
            for instr in curr.instructions.iter() {
                if instr
                    .regDefs()
                    .iter()
                    .any(|r| [SPL, SP, ESP, RSP].contains(r))
                {
                    requires_frame = true;
                }
            }
        });
        requires_frame
    }

    /// Emits the frame setup/teardown.
    ///
    /// This will also retroactively replace all Ret instructions with a Jmp to the epilogue
    pub fn emit_frame(&mut self) {
        let body = self.getEntryPoint();

        // Create prologue
        let prologue = self.newNamedBlock("prologue");
        self.setEntryPoint(prologue);
        self.emit_prologue();
        self.setInsertPoint(prologue);
        self.emit(Jmp(body));
        self.addSuccessorsToCurrent(&[body]);
        self.addFallthrough(body);

        // Create epilogue
        let epilogue = self.newNamedBlock("epilogue");
        self.setInsertPoint(epilogue);
        self.emit_epilogue();

        let leaf_labels = self.find_leaf_blocks();
        for label in leaf_labels {
            // TODO: Idea: What if we just emit the epilogue itself at each leaf?
            // Perhaps a good heuristic here would be code size/cache alignment.
            // We can avoid unnecessary control flow at the expense of bigger code

            let leaf = self.blocks.get_mut(&label).unwrap();
            leaf.delete_terminator();
            leaf.instructions.push(Jmp(epilogue));
            self.addSuccessorsTo(label, &[epilogue]);
        }
    }

    fn emit_prologue(&mut self) {
        self.emit(Push(Reg(RBP)));
        self.emit(Mov(Reg(RBP), Reg(RSP)));
    }

    fn emit_epilogue(&mut self) {
        self.emit(Mov(Reg(RBP), Reg(RSP)));
        self.emit(Pop(Reg(RBP)));
        self.emit(Ret);
    }
}
