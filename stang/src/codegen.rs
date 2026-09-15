use crate::IRs::tir::*;
use crate::ast::{BinOp, QualType};
use crate::die;
use crate::sema::CompilerContext;
use crate::translation_unit::{LoopLabels, SymbolKind, qtype};

use IRInstr::*;
use shrimple::comment;
use shrimple::stir::builder::*;
use shrimple::stir::isa::*;

impl TirFunction {
    pub fn codegen(mut self, ctx: &mut CompilerContext) -> IRFunction {
        let irrty = match self.return_type.as_ref() {
            QualType::Void => IRType::I32,
            x => x.toIRType(),
        };

        let mut function =
            IRFunction::new(self.name.inner.to_string(), self.return_type.toIRType(), ());

        self.codegen_locals(ctx, &mut function);
        self.body.take().unwrap().codegen(ctx, &mut function);
        let default_return = (*self.return_type == QualType::Void).then_some(Retv);
        if !function.verify(default_return) {
            die!(
                "Function {} expected to return {}, but not all paths return a value: {}",
                self.name.inner,
                self.return_type,
                self.name.span,
            )
        }
        function
    }

    fn codegen_locals(&mut self, ctx: &mut CompilerContext, function: &mut IRFunction) {
        let mut args = vec![];
        let mut locals = vec![];

        for symbol in self.local_symbols.iter() {
            let info = ctx.lookup_symbol(symbol);
            match info.kind {
                SymbolKind::Local => locals.push(symbol.clone()),
                SymbolKind::Arg(_) => args.push(symbol.clone()),
                SymbolKind::Global => {}
                SymbolKind::Function => {}
            }
        }

        // Sort argument symbols by their index
        args.sort_by_key(|s| {
            let info = ctx.lookup_symbol_mut(s);
            let SymbolKind::Arg(i) = info.kind else {
                unreachable!()
            };
            i
        });

        // Primitive args are alloca'd and filled normally, while structs are implicitly passed as pointers
        for symbol in args {
            let ty = ctx.lookup_symbol_mut(&symbol).ty.clone();
            let val = match ty.as_ref() {
                QualType::Struct { .. } => {
                    let arg = function.createVReg(IRType::Ptr);
                    function.addArg(arg);
                    arg
                }
                ty => {
                    let dst = function.createVReg(IRType::Ptr);
                    let irty = ty.toIRType();
                    let arg = function.createVReg(irty);
                    function.emit(Alloca(irty, dst));
                    function.addArg(arg);
                    function.emit(Store(irty, dst, arg));
                    dst
                }
            };
            comment!(function, "Argument {symbol} lives in {val}");
            ctx.lookup_symbol_mut(&symbol).value = Some(val);
        }

        // Sort locals by their local variable name
        locals.sort_by_key(|symbol| ctx.lookup_symbol_mut(symbol).raw_name.inner.clone());

        // Local variables are alloca'd but not initialized to anything
        for symbol in locals {
            let ty = ctx.lookup_symbol_mut(&symbol).ty.clone();
            let val = match ty.as_ref() {
                QualType::Struct { .. } => {
                    todo!("Figure out how to alloca aggregate types")
                }
                ty => {
                    let dst = function.createVReg(IRType::Ptr);
                    let irty = ty.toIRType();
                    function.emit(Alloca(irty, dst));
                    dst
                }
            };
            comment!(function, "Local {symbol} lives in {val}");
            ctx.lookup_symbol_mut(&symbol).value = Some(val);
        }
    }
}

impl TirStmt {
    pub fn codegen(&self, ctx: &mut CompilerContext, function: &mut IRFunction) {
        match self {
            TirStmt::While { cond, body } => {
                let cond_block = function.newNamedBlock("loopcond");
                let body_block = function.newNamedBlock("loopbody");
                let end_block = function.newNamedBlock("loopend");

                // Finish current block, add cond as successor
                function.emit(Jmp(cond_block));
                function.addSuccessorsToCurrent(&[cond_block]);

                // Codegen the cond expr
                function.setInsertPoint(cond_block);
                let cond_val = cond.codegen(ctx, function);

                function.emit(Br(cond_val, body_block, end_block));
                function.addSuccessorsToCurrent(&[body_block, end_block]);

                // Codegen the body stmt
                ctx.loop_labels.push(LoopLabels {
                    cond_block,
                    end_block,
                });
                function.setInsertPoint(body_block);
                body.codegen(ctx, function);
                ctx.loop_labels.pop();

                // Body always jumps to cond
                function.emit(Jmp(cond_block));
                function.addSuccessorsToCurrent(&[cond_block]);

                // Enter the post-loop block
                function.setInsertPoint(end_block);
            }
            TirStmt::Continue => {
                let Some(LoopLabels { cond_block, .. }) = ctx.loop_labels.last().copied() else {
                    die!("Continue statements can only be called within loops.");
                };
                function.emit(Jmp(cond_block));
            }
            TirStmt::Break => {
                let Some(LoopLabels { end_block, .. }) = ctx.loop_labels.last().copied() else {
                    die!("Continue statements can only be called within loops.");
                };
                function.emit(Jmp(end_block));
            }
            TirStmt::If { cond, then_, else_ } => {
                let then_block = function.newNamedBlock("then");
                let else_block = function.newNamedBlock("else");
                let endif_block = function.newNamedBlock("endif");

                // Codegen the cond expr
                let cond_val = cond.codegen(ctx, function);

                function.emit(Br(cond_val, then_block, else_block));
                function.addSuccessorsToCurrent(&[then_block, else_block]);

                // Codegen then stmt
                function.setInsertPoint(then_block);
                then_.codegen(ctx, function);

                // Then will jump to endif if not already terminated
                if !function.isCurrentTerminated() {
                    function.emit(Jmp(endif_block));
                    function.addSuccessorsToCurrent(&[endif_block]);
                }

                // Codegen else stmt
                function.setInsertPoint(else_block);
                else_.codegen(ctx, function);

                // Else always jumps to join
                if !function.isCurrentTerminated() {
                    function.emit(Jmp(endif_block));
                    function.addSuccessorsToCurrent(&[endif_block]);
                }

                // Enter the join block
                function.setInsertPoint(endif_block);
            }
            TirStmt::Return(ret_val) => match ret_val {
                Some(expr) => {
                    let val = expr.codegen(ctx, function);
                    let irty = *function.getReturnType();
                    function.emit(Ret(irty, val));
                }
                None => function.emit(Retv),
            },
            TirStmt::Block(stmts) => {
                for stmt in stmts.iter() {
                    stmt.codegen(ctx, function);
                }
            }
            TirStmt::Expr(expr) => {
                expr.codegen(ctx, function);
            }
        }
    }
}

impl TirExpr {
    pub fn codegen(&self, ctx: &mut CompilerContext, function: &mut IRFunction) -> IRValue {
        match &self.kind {
            TirExprKind::Num(n) => IRValue::Imm(*n, self.ty.toIRType()),
            TirExprKind::Bool(b) => IRValue::Imm((*b).into(), self.ty.toIRType()),
            TirExprKind::Store { ptr, val } => {
                let irty = val.ty.toIRType();
                let ptr = ptr.codegen(ctx, function);
                let val = val.codegen(ctx, function);
                function.emit(Store(irty, ptr, val));
                val
            }
            TirExprKind::Load { inner } => {
                let irty = inner.ty.get_pointee().toIRType();
                let ptr = inner.codegen(ctx, function);
                let val = function.createVReg(irty);
                function.emit(Load(irty, ptr, val));
                val
            }
            TirExprKind::ValueOf(symbol) => {
                let info = ctx.lookup_symbol(symbol);
                let irty = info.ty.toIRType();
                let ptr = info.value.expect("Symbol should have a value by now");
                let dst = function.createVReg(irty);
                function.emit(Load(irty, ptr, dst));
                dst
            }
            TirExprKind::AddrOf(symbol) => ctx.lookup_symbol(symbol).value.expect("No value"),
            TirExprKind::Un { op, rhs } => todo!(),
            TirExprKind::Bin { op, lhs, rhs } => {
                let ty = self.ty.clone();
                let irty = ty.toIRType();
                let lhs_val = lhs.codegen(ctx, function);
                let rhs_val = rhs.codegen(ctx, function);
                match op {
                    BinOp::Add => {
                        let result = function.createVReg(irty);
                        function.emit(Add(irty, result, lhs_val, rhs_val));
                        result
                    }
                    BinOp::Sub => {
                        let result = function.createVReg(irty);
                        function.emit(Sub(ty.toIRType(), result, lhs_val, rhs_val));
                        result
                    }
                    BinOp::PtrAdd => {
                        let dst = function.createVReg(irty);
                        if lhs.ty.is_pointer() {
                            let base_ty = lhs.ty.get_pointee().toIRType();
                            function.emit(Getaddr(dst, lhs_val, base_ty, rhs_val));
                        } else {
                            println!("rhs ty: {}", rhs.ty);
                            let base_ty = rhs.ty.get_pointee().toIRType();
                            function.emit(Getaddr(dst, rhs_val, base_ty, lhs_val));
                        }
                        dst
                    }
                    BinOp::PtrSub => todo!(),
                    BinOp::Mul => {
                        let result = function.createVReg(irty);
                        if lhs.ty.is_signed() {
                            function.emit(Smul(ty.toIRType(), result, lhs_val, rhs_val));
                        } else {
                            function.emit(Umul(ty.toIRType(), result, lhs_val, rhs_val));
                        }
                        result
                    }
                    BinOp::Div => {
                        let result = function.createVReg(irty);
                        if lhs.ty.is_signed() {
                            function.emit(Sdiv(ty.toIRType(), result, lhs_val, rhs_val));
                        } else {
                            function.emit(Udiv(ty.toIRType(), result, lhs_val, rhs_val));
                        }
                        result
                    }
                    BinOp::Eq => {
                        let result = function.createVReg(IRType::I1);
                        function.emit(Icmp(CmpOp::Eq, IRType::I1, result, lhs_val, rhs_val));
                        result
                    }
                    BinOp::Ne => {
                        let result = function.createVReg(IRType::I1);
                        let ty = qtype(&QualType::Bool);
                        function.emit(Icmp(CmpOp::Ne, ty.toIRType(), result, lhs_val, rhs_val));
                        result
                    }
                    BinOp::Le | BinOp::Lt | BinOp::Ge | BinOp::Gt => {
                        let result = function.createVReg(IRType::I1);
                        let ty = lhs.ty.clone();
                        let (signed, unsigned) = match op {
                            BinOp::Lt => (
                                Icmp(CmpOp::Slt, ty.toIRType(), result, lhs_val, rhs_val),
                                Icmp(CmpOp::Ult, ty.toIRType(), result, lhs_val, rhs_val),
                            ),
                            BinOp::Le => (
                                Icmp(CmpOp::Sle, ty.toIRType(), result, lhs_val, rhs_val),
                                Icmp(CmpOp::Ule, ty.toIRType(), result, lhs_val, rhs_val),
                            ),
                            BinOp::Gt => (
                                Icmp(CmpOp::Sgt, ty.toIRType(), result, lhs_val, rhs_val),
                                Icmp(CmpOp::Ugt, ty.toIRType(), result, lhs_val, rhs_val),
                            ),
                            BinOp::Ge => (
                                Icmp(CmpOp::Sge, ty.toIRType(), result, lhs_val, rhs_val),
                                Icmp(CmpOp::Uge, ty.toIRType(), result, lhs_val, rhs_val),
                            ),
                            _ => unreachable!(),
                        };
                        if lhs.ty.is_signed() {
                            function.emit(signed);
                        } else {
                            function.emit(unsigned);
                        }
                        result
                    }
                }
            }
            TirExprKind::Cast { target_ty, expr } => {
                let rhs_val = expr.codegen(ctx, function);

                let from_ty = expr.ty.toIRType();
                let to_ty = target_ty.toIRType();

                comment!(function, "Casting to {to_ty}");
                let dst = function.createVReg(to_ty);
                if target_ty.bits() < expr.ty.bits() {
                    function.emit(Trunc(to_ty, dst, from_ty, rhs_val))
                } else if target_ty.bits() > from_ty.bits() {
                    // i8 -> i32: sext
                    // u8 -> u32: zext
                    // i8 -> u32: zext
                    // u8 -> i32: sext
                    if target_ty.is_signed() {
                        function.emit(Sext(to_ty, dst, from_ty, rhs_val));
                    } else {
                        function.emit(Zext(to_ty, dst, from_ty, rhs_val));
                    }
                } else {
                    function.emit(Copy(to_ty, dst, rhs_val));
                }
                dst
            }
            TirExprKind::Call { callee, args } => {
                let callee_val = callee.codegen(ctx, function);
                let arg_values: Vec<_> = args.iter().map(|a| a.codegen(ctx, function)).collect();
                let irty = self.ty.toIRType();
                let dst = function.createVReg(irty);
                function.emit(Call(irty, dst, callee_val, arg_values.into()));
                dst
            }
        }
    }
}
