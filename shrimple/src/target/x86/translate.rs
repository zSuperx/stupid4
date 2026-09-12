///
/// Lowers from STIR to x86 MIR
///
use crate::comment;
use crate::stir::builder::IRFunction;
use crate::target::stir::isa::*;
use crate::target::x86::Backend;
use crate::target::x86::builder::x86Function;
use crate::target::x86::isa::*;
use std::collections::HashMap;

// Bring all x86 instructions into current namespace for convenience
use x86Instr::*;

/// Bumps `val` up to the next largest multiple of `n`
fn align_up_n(val: i128, n: i128) -> i128 {
    let rem = val % n;
    if rem > 0 { val + (n - rem) } else { val }
}

/// Brings `val` down to the next smallest multiple of `n`
fn align_down_n(val: i128, n: i128) -> i128 {
    let rem = val % n;
    val + rem
}

impl Backend {
    pub fn new() -> Self {
        Self::default()
    }

    fn createFrameSlot(
        &mut self,
        builder: &mut x86Function,
        value: &IRValue,
        ty: &IRType,
    ) -> x86Value {
        // TODO: Align each alloca based on its alignment
        // it's so wrong right now but i need to sleep
        let ty = LLType::fromIRType(ty);
        self.v_rsp -= ty.bytes() as i128;
        let old_v_rsp = self.v_rsp;
        self.v_rsp = align_down_n(self.v_rsp, ty.bytes() as i128);
        comment!(
            builder,
            "Virtual RSP aligned from {old_v_rsp} -> {}",
            self.v_rsp
        );

        builder.emit(Sub(RSP, x86Value::Imm(ty.bytes() as i128)));
        let slot = x86Value::memDisp(Reg::BP, self.v_rsp, ty);
        self.v2p.insert(*value, slot);
        slot
    }

    fn lowerToReg(&self, builder: &mut x86Function, value: &IRValue, ty: LLType) -> x86Value {
        match value {
            IRValue::Imm(i) => {
                let reg = x86Value::reg(builder.nextReg(), ty);
                builder.emit(Mov(reg, x86Value::Imm(*i)));
                reg
            }
            IRValue::Reg(r) => {
                if let Some(s) = self.v2p.get(value) {
                    // NOTE: If this was the n'th argument (where n > 6), this will return a memory
                    // value. Therefore, we can get illegal instructions like mov [...], [...]
                    //
                    // This should be fixed in a legalizer pass
                    *s
                } else {
                    x86Value::reg(Reg::Virt(*r), ty)
                }
            }
            IRValue::Ptr(r) => {
                if let Some(s) = self.v2p.get(value) {
                    assert!(s.is_mem());
                    // If this pointer is mapped to a physical address (i.e. [rbp - 8])
                    // we need to emit a lea instruction
                    let reg = x86Value::reg(builder.nextReg(), LLType::I64);
                    builder.emit(Lea(reg, *s));
                    return reg;
                } else {
                    // But if it's a new pointer, we don't have to bind it to a physical address
                    // We also don't need to emit a lea, since address of [%1] is just %1
                    x86Value::reg(Reg::Virt(*r), ty)
                }
            }
        }
    }

    fn lowerToPtr(&mut self, value: &IRValue, offset: i128, ty: LLType) -> x86Value {
        if let Some(s) = self.v2p.get(value) {
            return *s;
        }
        if let IRValue::Ptr(r) = value {
            x86Value::mem(Reg::Virt(*r), ty)
        } else {
            panic!("cant turn {value} into pointer")
        }
    }

    pub(crate) fn translate(&mut self, stir_function: &IRFunction) {
        // Create the function
        let rty = LLType::fromIRType(stir_function.getReturnType());
        let mut new_function = x86Function::new(stir_function.name.clone(), rty);
        for argty in stir_function.args.iter() {
            // self.ir_args.push(*argty);
        }

        // Machine Code Function
        let mcf = &mut new_function;
        mcf.setRegCount(stir_function.getRegCount());

        // Do a first pass to register all blocks in a map
        let mut block_map = HashMap::new();
        stir_function.dfs(|stir_builder, curr_id| {
            let curr = &stir_builder.blocks[&curr_id];
            let new = mcf.newNamedBlock(curr.label.name());
            block_map.insert(curr_id, new);
        });

        // Make the prologue the actual entrypoint
        let stir_ep = stir_function.getEntryPoint();
        let body = block_map[&stir_ep];

        // Create prologue
        let prologue = mcf.newNamedBlock("prologue");
        mcf.setEntryPoint(prologue);
        mcf.setInsertPoint(prologue);
        mcf.emit(Push(RBP));
        mcf.emit(Mov(RBP, RSP));
        mcf.emit(Jmp(body));
        mcf.addSuccessorsToCurrent(&[body]);
        mcf.addFallthrough(body);

        // Create epilogue
        let epilogue = mcf.newNamedBlock("epilogue");
        mcf.setInsertPoint(epilogue);
        mcf.emit(Mov(RSP, RBP));
        mcf.emit(Pop(RBP));
        mcf.emit(Ret);
        mcf.addFallthroughTo(body, epilogue);

        // Perform a visitor pass through the function and translate each block one at a time
        stir_function.dfs(|stir_function, curr_id| {
            // Map STIR BB to MC BB
            let curr = block_map[&curr_id];
            let block = &stir_function.blocks[&curr_id];
            mcf.setInsertPoint(curr);

            for instr in block.instructions.iter().chain(&block.terminator) {
                match instr {
                    IRInstr::Comment(s) => mcf.emit(Comment(s.clone())),
                    IRInstr::Jmp(b) => {
                        let b = block_map[b];
                        mcf.addSuccessorsToCurrent(&[b]);
                        mcf.emit(Jmp(b));
                    }
                    IRInstr::Store(ty, ptr, rs1) => {
                        let llty = LLType::fromIRType(ty);
                        let ptr = self.lowerToPtr(ptr, 0, llty);
                        let rs1 = self.lowerToReg(mcf, rs1, llty);
                        mcf.emit(Mov(ptr, rs1));
                    }
                    IRInstr::Load(ty, ptr, dst) => {
                        let llty = LLType::fromIRType(ty);
                        let ptr = self.lowerToPtr(ptr, 0, llty);
                        let dst = self.lowerToReg(mcf, dst, llty);
                        mcf.emit(Mov(dst, ptr));
                    }
                    IRInstr::Copy(ty, dst, rs1) => {
                        let llty = LLType::fromIRType(ty);
                        let dst = self.lowerToReg(mcf, dst, llty);
                        let rs1 = self.lowerToReg(mcf, rs1, llty);
                        mcf.emit(Mov(dst, rs1));
                    }
                    IRInstr::Sdiv(ty, dst, rs1, rs2)
                    | IRInstr::Smul(ty, dst, rs1, rs2)
                    | IRInstr::Sub(ty, dst, rs1, rs2)
                    | IRInstr::Udiv(ty, dst, rs1, rs2)
                    | IRInstr::Umul(ty, dst, rs1, rs2)
                    | IRInstr::Add(ty, dst, rs1, rs2) => {
                        let llty = LLType::fromIRType(ty);
                        let dst = self.lowerToReg(mcf, dst, llty);
                        let rs1 = self.lowerToReg(mcf, rs1, llty);
                        let rs2 = self.lowerToReg(mcf, rs2, llty);
                        mcf.emit(Mov(dst, rs1));
                        let op = match instr {
                            IRInstr::Sdiv(..) => Idiv,
                            IRInstr::Smul(..) => Imul,
                            IRInstr::Sub(..) => Sub,
                            // TODO: change these to Div/Mul?
                            // Bit awkward since they imply def & use of rax
                            IRInstr::Udiv(..) => Idiv,
                            IRInstr::Umul(..) => Imul,
                            IRInstr::Add(..) => Add,
                            _ => unreachable!(),
                        };
                        mcf.emit(op(dst, rs2));
                    }
                    IRInstr::Icmp(cmp, ty, dst, rs1, rs2) => {
                        let llty = LLType::fromIRType(ty);
                        let rs1 = self.lowerToReg(mcf, rs1, llty);
                        let rs2 = self.lowerToReg(mcf, rs2, llty);
                        mcf.emit(Cmp(rs1, rs2));
                        let flag = match cmp {
                            CmpOp::Slt | CmpOp::Ult => RFLAG::LT,
                            CmpOp::Ule | CmpOp::Sle => RFLAG::LE,
                            CmpOp::Ugt | CmpOp::Sgt => RFLAG::GT,
                            CmpOp::Uge | CmpOp::Sge => RFLAG::GE,
                            CmpOp::Eq => RFLAG::EQ,
                            CmpOp::Ne => RFLAG::NE,
                        };
                        self.v2p.insert(*dst, x86Value::CC(flag));
                    }
                    IRInstr::Br(cond, then_bb, else_bb) => {
                        let x86then = block_map[then_bb];
                        let x86else = block_map[else_bb];
                        mcf.addSuccessorsToCurrent(&[x86then]);
                        mcf.addFallthrough(x86else);
                        if let Some(phy) = self.v2p.get(cond) {
                            match phy {
                                x86Value::CC(rflag) => {
                                    let jcc = match rflag {
                                        RFLAG::LT => Jl,
                                        RFLAG::LE => Jle,
                                        RFLAG::GT => Jg,
                                        RFLAG::GE => Jge,
                                        RFLAG::EQ => Je,
                                        RFLAG::NE => Jne,
                                        RFLAG::Z => Jz,
                                        RFLAG::NZ => Jnz,
                                        RFLAG::O => Jo,
                                        RFLAG::NO => Jno,
                                    };
                                    mcf.emit(jcc(x86then));
                                }
                                _ => panic!("What"),
                            }
                        } else {
                            let cond = self.lowerToReg(mcf, cond, LLType::I8);
                            mcf.emit(Cmp(cond, x86Value::Imm(0)));
                            mcf.emit(Jnz(x86then));
                        }
                    }
                    // TODO: Improve the getaddr IR instruction to take multi-dimensional offsets
                    // Then just input those to memFull as scale, index, and disp
                    IRInstr::Getaddr(dst, base, ty, index) => {
                        let llty = LLType::fromIRType(ty);
                        let dst = self.lowerToReg(mcf, dst, LLType::I64);
                        let base = self.lowerToReg(mcf, base, LLType::I64);
                        let addr = match index {
                            IRValue::Reg(_) | IRValue::Ptr(_) => {
                                let index_val = self.lowerToReg(mcf, index, LLType::I64);
                                x86Value::memFull(
                                    base.getReg()[0],
                                    Some(index_val.getReg()[0]),
                                    llty.bytes(),
                                    0,
                                    llty,
                                )
                            }
                            IRValue::Imm(i) => {
                                x86Value::memFull(base.getReg()[0], None, llty.bytes(), *i, llty)
                            }
                        };
                        mcf.emit(Lea(dst, addr));
                    }
                    IRInstr::Alloca(ty, dst) => {
                        let dst = self.createFrameSlot(mcf, dst, ty);
                    }
                    IRInstr::Retv => {
                        mcf.addSuccessorsToCurrent(&[epilogue]);
                        mcf.emit(Jmp(epilogue));
                    }
                    IRInstr::Ret(ty, rs1) => {
                        let llty = LLType::fromIRType(ty);
                        let rs1 = self.lowerToReg(mcf, rs1, llty);

                        let rty_bits = mcf.getReturnType().bits();
                        let a = if rty_bits == 64 { RAX } else { EAX };

                        if ty.bits() >= a.ty().bits() {
                            mcf.emit(Mov(a, rs1));
                        } else {
                            mcf.emit(Movzx(a, rs1));
                        }

                        mcf.addSuccessorsToCurrent(&[epilogue]);
                        mcf.emit(Jmp(epilogue));
                    }
                    IRInstr::Trunc(to_ty, dst, from_ty, rs1) => {
                        let to_llty = LLType::fromIRType(to_ty);
                        let dst = self.lowerToReg(mcf, dst, to_llty);
                        let rs1 = self.lowerToReg(mcf, rs1, to_llty);
                        mcf.emit(Mov(dst, rs1));
                    }
                    IRInstr::Zext(to_ty, dst, from_ty, rs1)
                    | IRInstr::Sext(to_ty, dst, from_ty, rs1) => {
                        let to_llty = LLType::fromIRType(to_ty);
                        let dst = self.lowerToReg(mcf, dst, to_llty);
                        let from_llty = LLType::fromIRType(from_ty);
                        let rs1 = self.lowerToReg(mcf, rs1, from_llty);
                        match instr {
                            IRInstr::Zext(..) => mcf.emit(Movzx(dst, rs1)),
                            IRInstr::Sext(..) => mcf.emit(Movsx(dst, rs1)),
                            _ => unreachable!(),
                        }
                    }
                    IRInstr::Call(ty, dst, name, args) => todo!("Translate function calls"),
                }
            }
        });

        self.builder = Some(new_function);
    }
}
