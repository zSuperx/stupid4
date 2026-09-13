use crate::stir::builder::IRFunction;
use crate::stir::isa::*;
use crate::target::x86::builder::x86Function;
use crate::target::x86::isa::*;

impl x86Function {
    /// Scans the arguments of `stir_function` and maps them to physical registers by populating the
    /// `self.v2p` map. The mapping follows the x86_64 System V ABI specification.
    pub(crate) fn resolve_args(&mut self, stir_function: &IRFunction) {
        let mut registers = [Reg::DI, Reg::SI, Reg::D, Reg::C, Reg::R8, Reg::R9].iter();
        let mut used_stack_bytes = 0;
        let mut curr_reg = registers.next();
        // let mut new_args = vec![];
        for (i, (arg_val, arg_ty)) in stir_function.args.iter().enumerate() {
            if let IRType::Struct = arg_ty {
                todo!("System V ABI: Handle structs")
            } else {
                // Primitive type arguments each consume a single eightbyte register
                // If no registers are left, the argument shall live at [rbp + ...]
                let llty = LLType::fromIRType(arg_ty);
                let dst = match curr_reg {
                    Some(r) => x86Value::reg(*r, llty),
                    None => {
                        x86Value::memDisp(Reg::BP, 16 + (8 * i.saturating_sub(6) as i128), llty)
                    }
                };
                self.meta.v2p.insert(*arg_val, dst);
                curr_reg = registers.next();
            }
        }

        // stir_function.args = new_args;
    }

    pub(crate) fn lower_function_call(&mut self, instr: &IRInstr) {
        use x86Instr::*;
        let IRInstr::Call(ty, dst, callee, args) = instr else {
            panic!("Not a call instruction!");
        };
        let irty = LLType::fromIRType(ty);
        let dst = self.lowerToReg(dst, irty);

        let callee = self.lowerToReg(callee, LLType::I64);
        self.emit(Call(callee));
        self.emit(Mov(dst, x86Value::reg(Reg::A, irty)));
    }

    pub(crate) fn lower_return(&mut self, instr: &IRInstr) {
        use x86Instr::*;
        match instr {
            IRInstr::Ret(ty, rs1) => {
                let llty = LLType::fromIRType(ty);
                let rs1 = self.lowerToReg(rs1, llty);

                let rty_bits = self.getReturnType().bits();
                let a = if rty_bits == 64 { RAX } else { EAX };

                if ty.bits() >= a.ty().bits() {
                    self.emit(Mov(a, rs1));
                } else {
                    self.emit(Movzx(a, rs1));
                }
            }
            IRInstr::Retv => {
                self.emit(Mov(RAX, x86Value::Imm(0)));
            }
            _ => panic!("Not a return instruction!"),
        }
    }
}
