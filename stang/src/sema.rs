use crate::IRs::{hir::*, tir::*};
use crate::ast::*;
use crate::common::*;
use crate::translation_unit::{
    FunctionContext, SymbolInfo, SymbolKind, TranslationUnit, lookup_ident, lookup_symbol, qtype,
};
use std::rc::Rc;

impl HirFunction {
    /// Type checking a HIR Function produces a TIR Function, which holds the necessary context for
    /// code generation
    pub fn type_check(self, tu: &mut TranslationUnit) -> TirFunction {
        let return_type = tu.qualify_type(&self.return_type.inner);
        let symbol = tu.top_level_scope.get(&self.name.inner).cloned().unwrap();

        let mut tf = TirFunction {
            name: self.name,
            symbol,
            return_type,
            env: Env::with_base(tu.top_level_scope.clone()),
            loop_labels: Default::default(),
            loop_depth: Default::default(),
            symbol_table: Default::default(),
            symbol_counter: Default::default(),
            body: Default::default(),
        };

        let mut args = vec![];
        for (i, (raw_arg_name, raw_arg_type)) in self.args.into_iter().enumerate() {
            let arg_symbol = tf.add_local_symbol(
                raw_arg_name,
                tu.qualify_type(&raw_arg_type.inner),
                SymbolKind::Arg(i),
            );
            args.push(arg_symbol)
        }

        let body = self.body.type_check(tu, &mut tf);
        tf.body = Some(body);
        tf
    }
}

impl Spanned<HirStmt> {
    // Type checking a HIR statement requires the context of its parent function
    pub fn type_check(&self, tu: &mut TranslationUnit, tf: &mut TirFunction) -> TirStmt {
        let span = self.span;
        match &self.inner {
            HirStmt::LetDecl { name, ty } => {
                let Some(user_ty) = ty else {
                    die!("Uninitialized variables must be declared with a type {span}");
                };
                let qty = tu.qualify_type(&user_ty.inner);
                let symbol = tf.add_local_symbol(name.clone(), qty, SymbolKind::Local);
                // TODO: add a no-op or empty stmt
                TirStmt::Block(vec![])
            }
            HirStmt::LetAssign {
                name: lhs,
                ty,
                value,
            } => {
                let desired_type = ty.as_ref().map(|t| tu.qualify_type(&t.inner));
                let checked_val = value.type_check_rvalue(tu, tf, desired_type.clone());
                if let Some(lhs_ty) = desired_type
                    && lhs_ty != checked_val.ty
                {
                    die!("Expected type {lhs_ty} but got {}: {span}", checked_val.ty);
                }

                let rhs_ty = if let QualType::Function { .. } = checked_val.ty.as_ref() {
                    qtype(&QualType::Pointer(checked_val.ty.clone()))
                } else {
                    checked_val.ty.clone()
                };
                let symbol = tf.add_local_symbol(lhs.clone(), rhs_ty.clone(), SymbolKind::Local);
                let expr = {
                    let ty = rhs_ty.clone();
                    let kind = TirExprKind::Store {
                        ptr: Box::new(lookup_symbol(tu, tf, symbol).create_addr_of()),
                        val: Box::new(checked_val),
                    };
                    TirExpr::new(kind, ty)
                };
                TirStmt::Expr(expr)
            }
            HirStmt::While { cond, body } => {
                let hint = qtype(&QualType::Bool);
                let checked_cond = cond.type_check_rvalue(tu, tf, Some(hint));
                let cond_ty = checked_cond.ty.clone();
                if *cond_ty != QualType::Bool {
                    die!("Type mismatch. Expected bool but got {cond_ty}: {span}",)
                }

                tf.env.push_scope();
                tf.loop_depth += 1;
                let checked_body = body.type_check(tu, tf);
                tf.loop_depth -= 1;
                tf.env.pop_scope();

                TirStmt::While {
                    cond: checked_cond,
                    body: Box::new(checked_body),
                }
            }
            HirStmt::Continue => {
                if tf.loop_depth == 0 {
                    die!("continue statements can only be called within loops");
                }
                TirStmt::Continue
            }
            HirStmt::Break => {
                if tf.loop_depth == 0 {
                    die!("break statements can only be called within loops");
                }
                TirStmt::Break
            }
            HirStmt::If { cond, then_, else_ } => {
                let hint = qtype(&QualType::Bool);
                let checked_cond = cond.type_check_rvalue(tu, tf, Some(hint));
                let cond_ty = checked_cond.ty.clone();
                if *cond_ty != QualType::Bool {
                    die!("Type mismatch. Expected bool but got {cond_ty}: {span}",)
                }
                let checked_then = Box::new(then_.type_check(tu, tf));
                let checked_else = Box::new(else_.type_check(tu, tf));
                TirStmt::If {
                    cond: checked_cond,
                    then_: checked_then,
                    else_: checked_else,
                }
            }
            HirStmt::Return(expr) => {
                let ret_val = if let Some(expr) = expr {
                    let checked_expr = expr.type_check_rvalue(tu, tf, Some(tf.return_type.clone()));
                    if checked_expr.ty != tf.return_type {
                        die!(
                            "Function {} expected return type {}, but got {}: {span}",
                            tf.name,
                            tf.return_type,
                            checked_expr.ty,
                        );
                    }
                    Some(checked_expr)
                } else {
                    if *tf.return_type != QualType::Void {
                        die!(
                            "Function {} expected return type {}, but got void: {span}",
                            tf.name,
                            tf.return_type,
                        )
                    }
                    None
                };
                TirStmt::Return(ret_val)
            }
            HirStmt::Block(stmts) => {
                TirStmt::Block(stmts.into_iter().map(|s| s.type_check(tu, tf)).collect())
            }
            HirStmt::Expr(stmt) => TirStmt::Expr(stmt.type_check_rvalue(tu, tf, None)),
        }
    }
}

impl Spanned<HirExpr> {
    pub fn type_check_lvalue(
        &self,
        tu: &mut TranslationUnit,
        tf: &mut TirFunction,
        hint: Option<Rc<QualType>>,
    ) -> TirExpr {
        let span = self.span;
        match &self.inner {
            HirExpr::Ident(i) => {
                let Some(symbol) = tf.env.get(&i) else {
                    die!("Use of undefined variable: {span}")
                };
                let kind = TirExprKind::AddrOf(symbol.clone());
                let inner_ty = lookup_symbol(tu, tf, symbol).ty.clone();
                let ty = qtype(&QualType::Pointer(inner_ty));
                TirExpr::new(kind, ty)
            }
            HirExpr::Index { base, index } => {
                let checked_base = base.type_check_rvalue(tu, tf, hint);
                let checked_index = index.type_check_rvalue(tu, tf, Some(qtype(&QualType::U64)));
                if !checked_base.ty.is_pointer() {
                    die!(
                        "Can't index into non-pointer type {}: {base}",
                        checked_base.ty
                    )
                }

                if !checked_index.ty.is_integral() {
                    die!(
                        "Can't index using non-integer type {}: {index}",
                        checked_index.ty
                    )
                }

                let ty = checked_base.ty.clone();
                let kind = TirExprKind::Bin {
                    op: BinOp::PtrAdd,
                    lhs: Box::new(checked_base),
                    rhs: Box::new(checked_index),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::Deref { inner } => {
                let ptr = inner.type_check_rvalue(tu, tf, hint);
                if !ptr.ty.is_pointer() {
                    die!("Cannot dereference non-pointer type {}: {span}", ptr.ty);
                }
                ptr
            }
            HirExpr::Field { base, field } => todo!(),
            _ => die!("Invalid LVALUE expression: {span}"),
        }
    }

    pub fn type_check_rvalue(
        &self,
        tu: &mut TranslationUnit,
        tf: &mut TirFunction,
        hint: Option<Rc<QualType>>,
    ) -> TirExpr {
        let span = self.span;
        match &self.inner {
            HirExpr::Void => todo!(),
            HirExpr::Num(int_str) => {
                let ty = match hint {
                    Some(hint_id) => {
                        if hint_id.is_integral() || hint_id.is_pointer() {
                            hint_id
                        } else {
                            qtype(&QualType::I32)
                        }
                    }
                    None => qtype(&QualType::I32),
                };
                let result = match ty.as_ref() {
                    QualType::I8 => int_str.parse::<i8>().map(|i| i as i128),
                    QualType::U8 => int_str.parse::<u8>().map(|i| i as i128),
                    QualType::I16 => int_str.parse::<i16>().map(|i| i as i128),
                    QualType::U16 => int_str.parse::<u16>().map(|i| i as i128),
                    QualType::I32 => int_str.parse::<i32>().map(|i| i as i128),
                    QualType::U32 => int_str.parse::<u32>().map(|i| i as i128),
                    QualType::I64 => int_str.parse::<i64>().map(|i| i as i128),
                    QualType::U64 => int_str.parse::<u64>().map(|i| i as i128),
                    QualType::Pointer(..) if int_str.parse::<i32>().is_ok_and(|x| x == 0) => Ok(0),
                    _ => {
                        die!("`{int_str}` could not be parsed as a {ty}");
                    }
                };
                let Ok(num) = result else {
                    die!("`{int_str}` could not be parsed as a `{ty}`: {span}");
                };
                let kind = TirExprKind::Num(num);
                TirExpr::new(kind, ty)
            }
            HirExpr::Bool(b) => {
                let ty = qtype(&QualType::Bool);
                let kind = TirExprKind::Bool(*b);
                TirExpr::new(kind, ty)
            }
            HirExpr::Ident(i) => {
                let Some(info) = lookup_ident(tu, tf, i) else {
                    die!("Use of undefined variable {i}: {span}")
                };

                match info.ty.as_ref() {
                    QualType::Function { .. } => {
                        let kind = TirExprKind::AddrOf(info.symbol.clone());
                        let ty = qtype(&QualType::Pointer(info.ty.clone()));
                        TirExpr::new(kind, ty)
                    }
                    _ => {
                        let kind = TirExprKind::ValueOf(info.symbol.clone());
                        TirExpr::new(kind, info.ty.clone())
                    }
                }
            }
            HirExpr::Assign { lhs, rhs } => {
                // This should become a Store?
                // Check the LHS as an LVALUE. It must be a storage location
                let checked_lhs = lhs.type_check_lvalue(tu, tf, None);
                let lhs_ty = checked_lhs.ty.get_pointee().clone();

                // RHS can be anything
                let checked_rhs = rhs.type_check_rvalue(tu, tf, Some(lhs_ty.clone()));
                let rhs_ty = if let QualType::Function { .. } = checked_rhs.ty.as_ref() {
                    qtype(&QualType::Pointer(checked_rhs.ty.clone()))
                } else {
                    checked_rhs.ty.clone()
                };

                // Since LHS is a storage location, it's technically a pointer
                // so the RHS type should match whatever LHS is pointing to
                if lhs_ty != rhs_ty {
                    die!("Cannot assign a `{rhs_ty}` to `{lhs_ty}`: {span}")
                }

                // x = y = z should be possible
                let ty = checked_rhs.ty.clone();
                let kind = TirExprKind::Store {
                    ptr: Box::new(checked_lhs),
                    val: Box::new(checked_rhs),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::AddrOf { inner } => inner.type_check_lvalue(tu, tf, None),
            HirExpr::SizeOfTy { ty } => {
                let qt = tu.qualify_type(&ty.inner);
                let kind = TirExprKind::Num(qt.bytes() as i128);
                let ty = qtype(&QualType::U64);
                TirExpr::new(kind, ty)
            }
            HirExpr::SizeOfExpr { expr } => {
                let kind =
                    TirExprKind::Num(expr.type_check_rvalue(tu, tf, None).ty.bytes() as i128);
                let ty = qtype(&QualType::U64);
                TirExpr::new(kind, ty)
            }
            HirExpr::Deref { inner } => {
                let ptr = inner.type_check_rvalue(tu, tf, None);
                let ty = ptr.ty.get_pointee();
                println!("Dereferencing {} to {}", ptr.ty, ty);
                let kind = TirExprKind::Load {
                    inner: Box::new(ptr),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::Index { base, index } => {
                let base = base.type_check_rvalue(tu, tf, None);
                if !base.ty.is_pointer() {
                    die!("Cannot index into non-pointer type {}: {span}", base.ty);
                }
                let hint = qtype(&QualType::U64);
                let index = index.type_check_rvalue(tu, tf, Some(hint));
                let ty = base.ty.get_pointee();

                let inner = {
                    let ty = base.ty.clone();
                    let kind = TirExprKind::Bin {
                        op: BinOp::PtrAdd,
                        lhs: Box::new(base),
                        rhs: Box::new(index),
                    };
                    TirExpr::new(kind, ty)
                };

                let kind = TirExprKind::Load {
                    inner: Box::new(inner),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::Field { base, field } => todo!(),
            HirExpr::Un { op, rhs } => {
                let checked_rhs = rhs.type_check_rvalue(tu, tf, hint);
                let rhs_ty = checked_rhs.ty.clone();
                let ty = match op {
                    UnOp::Not => {
                        if *rhs_ty != QualType::Bool {
                            die!("Cannot logically not a {rhs_ty}: {span}")
                        }
                        rhs_ty
                    }
                    UnOp::Neg => {
                        if !rhs_ty.is_signed() {
                            die!("Cannot negate a {rhs_ty}: {span}")
                        }
                        rhs_ty
                    }
                };
                let kind = TirExprKind::Un {
                    op: *op,
                    rhs: Box::new(checked_rhs),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::Bin { op, lhs, rhs } => {
                enum BinType {
                    Integral,
                    Pointer,
                    Other,
                }

                /// Helper to avoid making calls to `ty.is_pointer()` or `ty.is_integral()`
                fn classify_type(ty: &QualType) -> BinType {
                    if ty.is_pointer() {
                        BinType::Pointer
                    } else if ty.is_integral() {
                        BinType::Integral
                    } else {
                        BinType::Other
                    }
                }

                let lhs = lhs.type_check_rvalue(tu, tf, hint);
                let lhs_class = classify_type(lhs.ty.as_ref());
                let rhs = rhs.type_check_rvalue(tu, tf, Some(lhs.ty.clone()));
                let rhs_class = classify_type(rhs.ty.as_ref());

                let (op, ty) = match (lhs_class, op, rhs_class) {
                    // ptr - ptr = int
                    (BinType::Pointer, BinOp::Sub, BinType::Pointer) if lhs.ty == rhs.ty => {
                        (BinOp::Sub, lhs.ty.clone())
                    }
                    // ptr + int = ptr
                    (BinType::Pointer, BinOp::Add, BinType::Integral) => {
                        (BinOp::PtrAdd, lhs.ty.clone())
                    }
                    // ptr - int = ptr
                    (BinType::Pointer, BinOp::Sub, BinType::Integral) => {
                        (BinOp::PtrSub, lhs.ty.clone())
                    }
                    // int + ptr = ptr
                    (BinType::Integral, BinOp::Add, BinType::Pointer) => {
                        (BinOp::PtrAdd, rhs.ty.clone())
                    }
                    // int +,-,/,* int = int
                    (BinType::Integral, op, BinType::Integral)
                        if lhs.ty == rhs.ty && op.is_arithmetic() =>
                    {
                        (*op, lhs.ty.clone())
                    }
                    // int >,>=,<,<= int = bool, ptr >,>=,<,<= ptr = bool
                    (
                        BinType::Integral | BinType::Pointer,
                        op,
                        BinType::Integral | BinType::Pointer,
                    ) if lhs.ty == rhs.ty && op.is_ordered() => (*op, qtype(&QualType::Bool)),
                    // any == any = bool, any != any = bool
                    (_, BinOp::Eq | BinOp::Ne, _) if lhs.ty == rhs.ty => {
                        (*op, qtype(&QualType::Bool))
                    }
                    _ => die!(
                        "Unsupported operation `{op}` between {} and {}",
                        lhs.ty,
                        rhs.ty
                    ),
                };
                let kind = TirExprKind::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::Cast { target_ty, rhs } => {
                // TODO: enforce type casting rules
                // Casting should be valid between:
                // - Same sized types (this means all pointers can be cast to and from each other)
                // - Any primitive with any other primitive
                let checked_ty = tu.qualify_type(&target_ty.inner);
                println!("Casting to {checked_ty}");
                let checked_rhs = rhs.type_check_rvalue(tu, tf, None);
                let kind = TirExprKind::Cast {
                    target_ty: checked_ty.clone(),
                    expr: Box::new(checked_rhs),
                };
                TirExpr::new(kind, checked_ty)
            }
            HirExpr::Call { callee, args } => {
                enum CallKind {
                    Direct,
                    Indirect,
                }

                if let HirExpr::Ident(name) = callee.inner.clone() {
                    let Some(info) = lookup_ident(tu, tf, &name) else {
                        die!("Use of undefined variable {name}: {span}")
                    };

                    let symbol = info.symbol.clone();

                    if let QualType::Function {
                        arg_types,
                        return_type,
                    } = info.ty.as_ref()
                    {
                        if arg_types.len() != args.len() {
                            die!(
                                "Function expects {} args but got {}: {span}",
                                arg_types.len(),
                                args.len()
                            )
                        }
                        let mut checked_args = vec![];
                        let ty = return_type.clone();
                        for (arg, expected_arg_type) in args.iter().zip(arg_types.clone().iter()) {
                            let checked_arg =
                                arg.type_check_rvalue(tu, tf, Some(expected_arg_type.clone()));
                            if checked_arg.ty != *expected_arg_type {
                                die!(
                                    "Incorrect argument passed to function. Expected {expected_arg_type} but got {}: {}",
                                    checked_arg.ty,
                                    arg.span
                                )
                            }
                            checked_args.push(checked_arg);
                        }
                        let kind = TirExprKind::DirectCall {
                            callee: symbol,
                            args: checked_args,
                        };
                        return TirExpr::new(kind, ty);
                    }
                }
                let checked_callee = callee.type_check_rvalue(tu, tf, None);
                match checked_callee.ty.as_ref() {
                    QualType::Pointer(f) => {
                        if let QualType::Function {
                            arg_types,
                            return_type,
                        } = f.as_ref()
                        {
                            let mut checked_args = vec![];
                            let ty = return_type.clone();
                            for (arg, expected_arg_type) in
                                args.iter().zip(arg_types.clone().iter())
                            {
                                let checked_arg =
                                    arg.type_check_rvalue(tu, tf, Some(expected_arg_type.clone()));
                                if checked_arg.ty != *expected_arg_type {
                                    die!(
                                        "Incorrect argument passed to function. Expected {expected_arg_type} but got {}: {}",
                                        checked_arg.ty,
                                        arg.span
                                    )
                                }
                                checked_args.push(checked_arg);
                            }
                            let kind = TirExprKind::IndirectCall {
                                callee: Box::new(checked_callee),
                                args: checked_args,
                            };
                            return TirExpr::new(kind, ty);
                        }
                    }
                    _ => {}
                }
                die!(
                    "Cannot call type {} as it is not a function or a pointer to a function: {span}",
                    checked_callee.ty
                );
            }
        }
    }
}
