///
/// Lowers from STIR to x86 MIR
///
use crate::comment;
use crate::stir::builder::IRFunction;
use crate::target::stir::isa::*;
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

impl x86Function {
    pub fn createFrameSlot(&mut self, value: &IRValue, ty: &IRType) -> x86Value {
        // TODO: Align each alloca based on its alignment
        // it's so wrong right now but i need to sleep
        let ty = LLType::fromIRType(ty);
        self.meta.v_rsp -= ty.bytes() as i128;
        let old_v_rsp = self.meta.v_rsp;
        self.meta.v_rsp = align_down_n(self.meta.v_rsp, ty.bytes() as i128);
        comment!(
            self,
            "Virtual RSP aligned from {old_v_rsp} -> {}",
            self.meta.v_rsp
        );

        self.emit(Sub(RSP, x86Value::Imm(ty.bytes() as i128)));
        let slot = x86Value::memDisp(Reg::BP, self.meta.v_rsp, ty);
        self.meta.v2p.insert(*value, slot);
        slot
    }

    pub fn lowerToReg(&mut self, value: &IRValue, ty: LLType) -> x86Value {
        match value {
            IRValue::Imm(i) => {
                let reg = x86Value::reg(self.nextReg(), ty);
                self.emit(Mov(reg, x86Value::Imm(*i)));
                reg
            }
            IRValue::Sym(s) => x86Value::Sym(*s),
            IRValue::Reg(r) => {
                if let Some(s) = self.meta.v2p.get(value) {
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
                if let Some(s) = self.meta.v2p.get(value).copied() {
                    assert!(s.is_mem());
                    // If this pointer is mapped to a physical address (i.e. [rbp - 8])
                    // we need to emit a lea instruction
                    let reg = x86Value::reg(self.nextReg(), LLType::I64);
                    self.emit(Lea(reg, s));
                    reg
                } else {
                    // But if it's a new pointer, we don't have to bind it to a physical address
                    // We also don't need to emit a lea, since address of [%1] is just %1
                    x86Value::reg(Reg::Virt(*r), ty)
                }
            }
        }
    }

    pub fn lowerToPtr(&mut self, value: &IRValue, offset: i128, ty: LLType) -> x86Value {
        if let Some(s) = self.meta.v2p.get(value) {
            return *s;
        }
        match value {
            IRValue::Ptr(r) => x86Value::mem(Reg::Virt(*r), ty),
            IRValue::Sym(s) => x86Value::Sym(*s),
            _ => panic!("cant turn {value:?} into pointer"),
        }
    }

    pub(crate) fn translate(&mut self, stir_function: &IRFunction) {
        // Do a first pass to register all blocks in a map
        let mut block_map = HashMap::new();
        stir_function.dfs(|stir_builder, curr_id| {
            let curr = &stir_builder.blocks[&curr_id];
            let new = self.newNamedBlock(curr.label.name());
            block_map.insert(curr_id, new);
        });

        // Make the prologue the actual entrypoint
        let stir_ep = stir_function.getEntryPoint();
        let body = block_map[&stir_ep];

        // Create prologue
        let prologue = self.newNamedBlock("prologue");
        self.setEntryPoint(prologue);
        self.setInsertPoint(prologue);
        self.emit(Push(RBP));
        self.emit(Mov(RBP, RSP));
        self.emit(Jmp(body));
        self.addSuccessorsToCurrent(&[body]);
        self.addFallthrough(body);

        // Create epilogue
        let epilogue = self.newNamedBlock("epilogue");
        self.setInsertPoint(epilogue);
        self.emit(Mov(RSP, RBP));
        self.emit(Pop(RBP));
        self.emit(Ret);
        self.addFallthroughTo(body, epilogue);

        // Perform a visitor pass through the function and translate each block one at a time
        stir_function.dfs(|stir_function, curr_id| {
            // Map STIR BB to MC BB
            let curr = block_map[&curr_id];
            let block = &stir_function.blocks[&curr_id];
            self.setInsertPoint(curr);

            for instr in block.instructions.iter().chain(&block.terminator) {
                match instr {
                    IRInstr::Comment(s) => self.emit(Comment(s.clone())),
                    IRInstr::Jmp(b) => {
                        let b = block_map[b];
                        self.addSuccessorsToCurrent(&[b]);
                        self.emit(Jmp(b));
                    }
                    IRInstr::Store(ty, ptr, rs1) => {
                        let llty = LLType::fromIRType(ty);
                        let ptr = self.lowerToPtr(ptr, 0, llty);
                        let rs1 = self.lowerToReg(rs1, llty);
                        self.emit(Mov(ptr, rs1));
                    }
                    IRInstr::Load(ty, ptr, dst) => {
                        let llty = LLType::fromIRType(ty);
                        let ptr = self.lowerToPtr(ptr, 0, llty);
                        let dst = self.lowerToReg(dst, llty);
                        self.emit(Mov(dst, ptr));
                    }
                    IRInstr::Copy(ty, dst, rs1) => {
                        let llty = LLType::fromIRType(ty);
                        let dst = self.lowerToReg(dst, llty);
                        let rs1 = self.lowerToReg(rs1, llty);
                        self.emit(Mov(dst, rs1));
                    }
                    IRInstr::Sdiv(ty, dst, rs1, rs2)
                    | IRInstr::Smul(ty, dst, rs1, rs2)
                    | IRInstr::Sub(ty, dst, rs1, rs2)
                    | IRInstr::Udiv(ty, dst, rs1, rs2)
                    | IRInstr::Umul(ty, dst, rs1, rs2)
                    | IRInstr::Add(ty, dst, rs1, rs2) => {
                        let llty = LLType::fromIRType(ty);
                        let dst = self.lowerToReg(dst, llty);
                        let rs1 = self.lowerToReg(rs1, llty);
                        let rs2 = self.lowerToReg(rs2, llty);
                        self.emit(Mov(dst, rs1));
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
                        self.emit(op(dst, rs2));
                    }
                    IRInstr::Icmp(cmp, ty, dst, rs1, rs2) => {
                        let llty = LLType::fromIRType(ty);
                        let rs1 = self.lowerToReg(rs1, llty);
                        let rs2 = self.lowerToReg(rs2, llty);
                        self.emit(Cmp(rs1, rs2));
                        let flag = match cmp {
                            CmpOp::Slt | CmpOp::Ult => RFLAG::LT,
                            CmpOp::Ule | CmpOp::Sle => RFLAG::LE,
                            CmpOp::Ugt | CmpOp::Sgt => RFLAG::GT,
                            CmpOp::Uge | CmpOp::Sge => RFLAG::GE,
                            CmpOp::Eq => RFLAG::EQ,
                            CmpOp::Ne => RFLAG::NE,
                        };
                        self.meta.v2p.insert(*dst, x86Value::CC(flag));
                    }
                    IRInstr::Br(cond, then_bb, else_bb) => {
                        let x86then = block_map[then_bb];
                        let x86else = block_map[else_bb];
                        self.addSuccessorsToCurrent(&[x86then]);
                        self.addFallthrough(x86else);
                        if let Some(phy) = self.meta.v2p.get(cond) {
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
                                    self.emit(jcc(x86then));
                                }
                                _ => panic!("What"),
                            }
                        } else {
                            let cond = self.lowerToReg(cond, LLType::I8);
                            self.emit(Cmp(cond, x86Value::Imm(0)));
                            self.emit(Jnz(x86then));
                        }
                    }
                    // TODO: Improve the getaddr IR instruction to take multi-dimensional offsets
                    // Then just input those to memFull as scale, index, and disp
                    IRInstr::Getaddr(dst, base, ty, index) => {
                        let llty = LLType::fromIRType(ty);
                        let dst = self.lowerToReg(dst, LLType::I64);
                        let base = self.lowerToReg(base, LLType::I64);
                        let addr = match index {
                            IRValue::Reg(_) | IRValue::Ptr(_) => {
                                let index_val = self.lowerToReg(index, LLType::I64);
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
                            IRValue::Sym(s) => x86Value::Sym(*s),
                        };
                        self.emit(Lea(dst, addr));
                    }
                    IRInstr::Alloca(ty, dst) => {
                        let dst = self.createFrameSlot(dst, ty);
                    }
                    IRInstr::Retv => {
                        self.lower_return(instr);
                        self.addSuccessorsToCurrent(&[epilogue]);
                        self.emit(Jmp(epilogue));
                    }
                    IRInstr::Ret(ty, rs1) => {
                        self.lower_return(instr);
                        self.addSuccessorsToCurrent(&[epilogue]);
                        self.emit(Jmp(epilogue));
                    }
                    IRInstr::Trunc(to_ty, dst, from_ty, rs1) => {
                        let to_llty = LLType::fromIRType(to_ty);
                        let dst = self.lowerToReg(dst, to_llty);
                        let rs1 = self.lowerToReg(rs1, to_llty);
                        self.emit(Mov(dst, rs1));
                    }
                    IRInstr::Zext(to_ty, dst, from_ty, rs1)
                    | IRInstr::Sext(to_ty, dst, from_ty, rs1) => {
                        let to_llty = LLType::fromIRType(to_ty);
                        let dst = self.lowerToReg(dst, to_llty);
                        let from_llty = LLType::fromIRType(from_ty);
                        let rs1 = self.lowerToReg(rs1, from_llty);
                        match instr {
                            IRInstr::Zext(..) => self.emit(Movzx(dst, rs1)),
                            IRInstr::Sext(..) => self.emit(Movsx(dst, rs1)),
                            _ => unreachable!(),
                        }
                    }
                    IRInstr::Call(..) => self.lower_function_call(instr),
                }
            }
        });
    }
}
