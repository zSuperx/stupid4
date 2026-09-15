use crate::stir::builder::IRFunction;
use crate::stir::isa::*;
use crate::target::x86::builder::x86Function;
use crate::target::x86::isa::*;

impl x86Function {
    /// Scans the arguments of `stir_function` and maps them to physical registers by populating the
    /// `self.v2p` map. The mapping follows the x86_64 System V ABI specification.
    pub(crate) fn resolve_args(&mut self, stir_function: &IRFunction) {
        let mut registers = [RDI, RSI, RDX, RCX, R8, R9].iter();
        let mut curr_reg = registers.next();
        for (i, arg_val) in stir_function.args.iter().enumerate() {
            let (IRValue::Reg(_, arg_ty) | IRValue::Ptr(_, arg_ty)) = *arg_val else {
                panic!("Argument is not a register or pointer");
            };

            if let IRType::Struct = arg_ty {
                todo!("System V ABI: Handle structs")
            } else {
                // Primitive type arguments each consume a single eightbyte register
                // If no registers are left, the argument shall live at [rbp + ...]
                let llty = LLType::fromIRType(&arg_ty);
                let dst = match curr_reg {
                    Some(r) => x86Value::Reg(*r),
                    None => x86Value::memDisp(
                        RBP,
                        16 + (8 * i.saturating_sub(6) as i128),
                        llty,
                    ),
                };
                self.meta.v2p.insert(*arg_val, dst);
                curr_reg = registers.next();
            }
        }
    }

    pub(crate) fn lower_function_call(&mut self, instr: &IRInstr) {
        use x86Instr::*;
        let IRInstr::Call(ty, dst, callee, args) = instr else {
            panic!("Not a call instruction!");
        };
        let llty = LLType::fromIRType(ty);
        let dst = self.lowerToReg(dst);

        // While it is functionally correct,
        //
        // lea %tmp, [rel foo]
        // call %tmp
        //
        // is equivalent to
        //
        // call foo
        //
        // Therefore, we handle the callee directly since lowerToReg produces the top result 
        let callee = match callee {
            IRValue::Sym(s) => x86Value::Sym(*s),
            _ => self.lowerToReg(callee),
        };

        self.emit(Call(callee));
        let a = match llty {
            LLType::I64 => Reg(RAX),
            _ => Reg(EAX),
        };

        let a = match dst.get_type() {
            LLType::I64 => Reg(RAX),
            LLType::I32 => Reg(EAX),
            LLType::I16 => Reg(AX),
            LLType::I8 => Reg(AL),
            _ => unreachable!(),
        };
        self.emit(Mov(dst, a));
    }

    pub(crate) fn lower_return(&mut self, instr: &IRInstr) {
        use x86Instr::*;
        match instr {
            IRInstr::Ret(ty, rs1) => {
                let llty = LLType::fromIRType(ty);
                let rs1 = self.lowerToReg(rs1);

                let rty_bits = self.getReturnType().bits();
                let a = match llty {
                    LLType::I64 => Reg(RAX),
                    _ => Reg(EAX),
                };

                self.emit(Mov(a, rs1));
                self.emit(Ret);
            }
            IRInstr::Retv => {
                self.emit(Mov(x86Value::Reg(RAX), x86Value::Imm(0)));
                self.emit(Ret);
            }
            _ => panic!("Not a return instruction!"),
        }
    }
}
