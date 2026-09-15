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
    pub fn createFrameSlot(&mut self, ty: LLType) -> x86Value {
        self.meta.frame_slots.push(ty);
        Mem(AddressMode::FrameSlot { index: self.meta.frame_slots.len() - 1, ty })
    }

    pub fn lowerToReg(&mut self, value: &IRValue) -> x86Value {
        let ret = if let Some(x) = self.meta.v2p.get(value).copied() {
            match x {
                Reg(..) => x,
                Mem(..) | Sym(..) => self.materialize_address(x),
                _ => panic!("Cannot lower this value to a register {x:?}"),
            }
        } else {
            match value {
                IRValue::Imm(i, irtype) => {
                    let reg = self.createRegValue(irtype.into());
                    self.emit(Mov(reg, Imm(*i)));
                    self.meta.v2p.insert(*value, reg);
                    reg
                }
                IRValue::Reg(_, irtype) => {
                    let reg = self.createRegValue(irtype.into());
                    self.meta.v2p.insert(*value, reg);
                    reg
                }
                IRValue::Ptr(_, irtype) => {
                    let llty = irtype.into();
                    let mem = self.createMemValue(llty);
                    self.meta.v2p.insert(*value, mem);
                    self.materialize_address(mem)
                }
                IRValue::Sym(module_symbol) => {
                    let mem = Mem(AddressMode::Relative(*module_symbol));
                    self.materialize_address(mem)
                }
            }
        };
        ret
    }

    /// Takes a memory value (of the form [...]) and returns its address.
    ///
    /// For non-trivial addressing like [rbp - 8], an intermediate `lea` is emitted, but simple
    /// addresses like [rbp] directly return the inner base register.
    pub fn materialize_address(&mut self, memory_value: x86Value) -> x86Value {
        let reg = match memory_value {
            Mem(AddressMode::Direct {
                base,
                index: None,
                scale,
                disp: 0,
                ty,
            }) => Reg(base),
            Mem(..) => {
                let reg = self.createRegValue(LLType::I64);
                self.emit(Lea(reg, memory_value));
                reg
            }
            _ => panic!("Not a memory value! {memory_value:?}"),
        };
        comment!(self, "Materialized address: {memory_value} -> {reg}");
        reg
    }

    pub fn lowerToPtr(&mut self, value: &IRValue, offset: i128, ty: LLType) -> x86Value {
        if let Some(s) = self.meta.v2p.get(value).copied() {
            assert!(s.is_mem());
            s
        } else {
            let mem = match value {
                IRValue::Ptr(..) => self.createMemValue(ty),
                IRValue::Sym(s) => Mem(AddressMode::Relative(*s)),
                _ => panic!("cant turn {value:?} into pointer"),
            };
            self.meta.v2p.insert(*value, mem);
            mem
        }
    }

    pub(crate) fn translate(&mut self, stir_function: &IRFunction) {
        // Do a first pass to register all blocks in a map
        let mut block_map = HashMap::new();
        stir_function.dfs(|stir_function, curr_id| {
            let curr = &stir_function.blocks[&curr_id];
            let new = self.newNamedBlock(curr.label.name());
            block_map.insert(curr_id, new);
        });

        // NOTE: We don't emit the prologue/epilogue here. Instead, a later optimization pass
        // decides whether one is needed and only emits it if so. This lets our function avoid frame
        // setup/teardown when not necessary

        // Perform a visitor pass through the function and translate each block one at a time
        stir_function.dfs(|stir_function, ir_label| {
            let ir_block = &stir_function.blocks[&ir_label];

            // Map STIR BB to MC BB
            let mc_label = block_map[&ir_label];
            self.setInsertPoint(mc_label);

            if ir_label == stir_function.getEntryPoint() {
                self.setEntryPoint(mc_label);
            }

            for instr in ir_block.instructions.iter() {
                match instr {
                    IRInstr::Comment(s) => self.emit(Comment(s.clone())),
                    IRInstr::Alloca(ty, dst) => {
                        let mem = self.createFrameSlot(ty.into());
                        self.meta.v2p.insert(*dst, mem);
                    }
                    IRInstr::Store(ty, ptr, rs1) => {
                        let llty = ty.into();
                        let ptr = self.lowerToPtr(ptr, 0, llty);
                        let rs1 = self.lowerToReg(rs1);
                        self.emit(Mov(ptr, rs1));
                    }
                    IRInstr::Load(ty, ptr, vdst) => {
                        let llty = ty.into();
                        let ptr = self.lowerToPtr(ptr, 0, llty);
                        let dst = self.lowerToReg(vdst);
                        self.emit(Mov(dst, ptr));
                    }
                    IRInstr::Copy(ty, dst, rs1) => {
                        let dst = self.lowerToReg(dst);
                        let rs1 = self.lowerToReg(rs1);
                        self.emit(Mov(dst, rs1));
                    }
                    IRInstr::Sdiv(ty, dst, rs1, rs2)
                    | IRInstr::Smul(ty, dst, rs1, rs2)
                    | IRInstr::Sub(ty, dst, rs1, rs2)
                    | IRInstr::Udiv(ty, dst, rs1, rs2)
                    | IRInstr::Umul(ty, dst, rs1, rs2)
                    | IRInstr::Add(ty, dst, rs1, rs2) => {
                        let dst = self.lowerToReg(dst);
                        let rs1 = self.lowerToReg(rs1);
                        let rs2 = self.lowerToReg(rs2);
                        self.emit(Mov(dst, rs1));
                        let op = match instr {
                            IRInstr::Sdiv(..) => Idiv,
                            IRInstr::Smul(..) => Imul,
                            IRInstr::Sub(..) => Sub,
                            // TODO: change these to Div/Mul?
                            // Bit awkward since they imply def & use of rax/rdx
                            IRInstr::Udiv(..) => Idiv,
                            IRInstr::Umul(..) => Imul,
                            IRInstr::Add(..) => Add,
                            _ => unreachable!(),
                        };
                        self.emit(op(dst, rs2));
                    }
                    IRInstr::Icmp(cmp, ty, dst, rs1, rs2) => {
                        let rs1 = self.lowerToReg(rs1);
                        let rs2 = self.lowerToReg(rs2);
                        self.emit(Cmp(rs1, rs2));
                        let flag = match cmp {
                            CmpOp::Slt | CmpOp::Ult => RFLAG::LT,
                            CmpOp::Ule | CmpOp::Sle => RFLAG::LE,
                            CmpOp::Ugt | CmpOp::Sgt => RFLAG::GT,
                            CmpOp::Uge | CmpOp::Sge => RFLAG::GE,
                            CmpOp::Eq => RFLAG::EQ,
                            CmpOp::Ne => RFLAG::NE,
                        };
                        self.meta.v2p.insert(*dst, CC(flag));
                    }
                    IRInstr::Jmp(b) => {
                        let b = block_map[b];
                        self.addSuccessorsToCurrent(&[b]);
                        self.emit(Jmp(b));
                    }
                    IRInstr::Br(cond, then_bb, else_bb) => {
                        let x86then = block_map[then_bb];
                        let x86else = block_map[else_bb];
                        self.addSuccessorsToCurrent(&[x86then]);
                        self.addFallthrough(x86else);
                        if let Some(phy) = self.meta.v2p.get(cond) {
                            match phy {
                                CC(rflag) => {
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
                            let cond = self.lowerToReg(cond);
                            self.emit(Cmp(cond, Imm(0)));
                            self.emit(Jnz(x86then));
                        }
                    }
                    // TODO: Improve the getaddr IR instruction to take multi-dimensional offsets
                    // Then just input those to memFull as scale, index, and disp
                    IRInstr::Getaddr(dst, base, ty, index) => {
                        let llty: LLType = ty.into();
                        let dst = self.lowerToReg(dst);
                        let base = self.lowerToReg(base);
                        let addr = match index {
                            IRValue::Reg(..) | IRValue::Ptr(..) => {
                                let index_val = self.lowerToReg(index);
                                x86Value::memDirect(
                                    base.getReg()[0],
                                    Some(index_val.getReg()[0]),
                                    llty.bytes(),
                                    0,
                                    llty,
                                )
                            }
                            IRValue::Imm(i, _) => {
                                x86Value::memDirect(base.getReg()[0], None, llty.bytes(), *i, llty)
                            }
                            IRValue::Sym(s) => Mem(AddressMode::Relative(*s)),
                        };
                        self.emit(Lea(dst, addr));
                    }
                    IRInstr::Ret(..) | IRInstr::Retv => self.lower_return(instr),
                    IRInstr::Trunc(to_ty, dst, from_ty, rs1) => {
                        let dst = self.lowerToReg(dst);
                        let rs1 = self.lowerToReg(rs1);
                        self.emit(Mov(dst, rs1));
                    }
                    IRInstr::Zext(to_ty, dst, from_ty, rs1)
                    | IRInstr::Sext(to_ty, dst, from_ty, rs1) => {
                        let dst = self.lowerToReg(dst);
                        let rs1 = self.lowerToReg(rs1);
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
