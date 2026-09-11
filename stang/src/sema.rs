use crate::IRs::{hir::*, tir::*};
use crate::ast::*;
use crate::common::*;
use crate::translation_unit::{
    FunctionContext, SymbolInfo, SymbolKind, TranslationUnit, add_type, lookup_ident, lookup_symbol,
};

impl HirFunction {
    /// Type checking a HIR Function produces a TIR Function, which holds the necessary context for
    /// code generation
    pub fn type_check(self, tu: &mut TranslationUnit) -> TirFunction {
        let return_type = tu.resolve_type(self.return_type.inner);
        let symbol = tu.top_level_scope.get(self.name.inner).copied().unwrap();

        let mut tf = TirFunction {
            name: self.name,
            symbol,
            return_type,
            env: Default::default(),
            loop_labels: Default::default(),
            loop_depth: Default::default(),
            symbol_table: Default::default(),
            symbol_counter: Default::default(),
            body: Default::default(),
        };
        tf.env.push_filled_scope(tu.top_level_scope.clone());

        let mut args = vec![];
        for (i, (raw_arg_name, raw_arg_type)) in self.args.into_iter().enumerate() {
            let arg_symbol = tf.add_local_symbol(
                raw_arg_name,
                tu.resolve_type(raw_arg_type.inner),
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
            HirStmt::Let { lhs, ty, rhs } => {
                let desired_type = ty.map(|t| tu.resolve_type(t.inner));
                let checked_rhs = rhs.type_check_rvalue(tu, tf, desired_type);
                if let Some(lhs_ty) = desired_type
                    && lhs_ty != checked_rhs.ty
                {
                    die!("Expected type {lhs_ty} but got {}: {span}", checked_rhs.ty);
                }

                let rhs_ty = if let Type::Function { .. } = checked_rhs.ty.lookup() {
                    add_type(Type::Pointer(checked_rhs.ty))
                } else {
                    checked_rhs.ty
                };
                let symbol = tf.add_local_symbol(*lhs, rhs_ty, SymbolKind::Local);
                TirStmt::Let {
                    lhs: symbol,
                    rhs: checked_rhs,
                }
            }
            HirStmt::While { cond, body } => {
                let hint = add_type(Type::Bool);
                let checked_cond = cond.type_check_rvalue(tu, tf, Some(hint));
                let cond_ty = checked_cond.ty;
                if *cond_ty != Type::Bool {
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
                let hint = add_type(Type::Bool);
                let checked_cond = cond.type_check_rvalue(tu, tf, Some(hint));
                let cond_ty = checked_cond.ty;
                if *cond_ty != Type::Bool {
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
                    let checked_expr = expr.type_check_rvalue(tu, tf, Some(tf.return_type));
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
                    if *tf.return_type != Type::Void {
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
        hint: Option<TypeId>,
    ) -> TirExpr {
        let span = self.span;
        match &self.inner {
            HirExpr::Ident(i) => {
                let Some(symbol) = tf.env.get(&i) else {
                    die!("Use of undefined variable: {span}")
                };
                let kind = TirExprKind::AddrOf(symbol);
                let inner_ty = lookup_symbol(tu, tf, symbol).ty;
                let ty = add_type(Type::Pointer(inner_ty));
                TirExpr::new(kind, ty)
            }
            HirExpr::Index { base, index } => {
                let checked_base = base.type_check_rvalue(tu, tf, hint);
                let checked_index = index.type_check_rvalue(tu, tf, Some(add_type(Type::U64)));
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

                let ty = checked_base.ty;
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
        hint: Option<TypeId>,
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
                            add_type(Type::I32)
                        }
                    }
                    None => add_type(Type::I32),
                };
                let result = match ty.lookup() {
                    Type::I8 => int_str.parse::<i8>().map(|i| i as i128),
                    Type::U8 => int_str.parse::<u8>().map(|i| i as i128),
                    Type::I16 => int_str.parse::<i16>().map(|i| i as i128),
                    Type::U16 => int_str.parse::<u16>().map(|i| i as i128),
                    Type::I32 => int_str.parse::<i32>().map(|i| i as i128),
                    Type::U32 => int_str.parse::<u32>().map(|i| i as i128),
                    Type::I64 => int_str.parse::<i64>().map(|i| i as i128),
                    Type::U64 => int_str.parse::<u64>().map(|i| i as i128),
                    Type::Pointer(..) if int_str.parse::<i32>().is_ok_and(|x| x == 0) => Ok(0),
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
                let ty = add_type(Type::Bool);
                let kind = TirExprKind::Bool(*b);
                TirExpr::new(kind, ty)
            }
            HirExpr::Ident(i) => {
                let Some(info) = lookup_ident(tu, tf, i) else {
                    die!("Use of undefined variable {i}: {span}")
                };

                match info.ty.lookup() {
                    Type::Function { .. } => {
                        let kind = TirExprKind::AddrOf(info.symbol);
                        let ty = add_type(Type::Pointer(info.ty));
                        TirExpr::new(kind, ty)
                    }
                    _ => {
                        let kind = TirExprKind::ValueOf(info.symbol);
                        TirExpr::new(kind, info.ty)
                    }
                }
            }
            HirExpr::Assign { lhs, rhs } => {
                // This should become a Store?
                // Check the LHS as an LVALUE. It must be a storage location
                let checked_lhs = lhs.type_check_lvalue(tu, tf, None);
                let lhs_ty = checked_lhs.ty.get_pointee();

                // RHS can be anything
                let checked_rhs = rhs.type_check_rvalue(tu, tf, Some(lhs_ty));
                let rhs_ty = if let Type::Function { .. } = checked_rhs.ty.lookup() {
                    add_type(Type::Pointer(checked_rhs.ty))
                } else {
                    checked_rhs.ty
                };

                // Since LHS is a storage location, it's technically a pointer
                // so the RHS type should match whatever LHS is pointing to
                if lhs_ty != rhs_ty {
                    die!("Cannot assign a `{rhs_ty}` to `{lhs_ty}`: {span}")
                }

                // x = y = z should be possible
                let ty = checked_rhs.ty;
                let kind = TirExprKind::Store {
                    ptr: Box::new(checked_lhs),
                    val: Box::new(checked_rhs),
                };
                TirExpr::new(kind, ty)
            }
            HirExpr::AddrOf { inner } => inner.type_check_lvalue(tu, tf, None),
            HirExpr::SizeOfTy { ty } => {
                let kind = TirExprKind::Num(ty.inner.bytes() as i128);
                let ty = add_type(Type::U64);
                TirExpr::new(kind, ty)
            }
            HirExpr::SizeOfExpr { expr } => {
                let kind =
                    TirExprKind::Num(expr.type_check_rvalue(tu, tf, None).ty.bytes() as i128);
                let ty = add_type(Type::U64);
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
                let hint = add_type(Type::U64);
                let index = index.type_check_rvalue(tu, tf, Some(hint));
                let ty = base.ty.get_pointee();

                let inner = {
                    let ty = base.ty;
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
                let rhs_ty = checked_rhs.ty;
                let ty = match op {
                    UnOp::Not => {
                        if *rhs_ty != Type::Bool {
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
                fn classify_type(ty: &Type) -> BinType {
                    if ty.is_pointer() {
                        BinType::Pointer
                    } else if ty.is_integral() {
                        BinType::Integral
                    } else {
                        BinType::Other
                    }
                }

                let lhs = lhs.type_check_rvalue(tu, tf, hint);
                let lhs_class = classify_type(lhs.ty.lookup());
                let rhs = rhs.type_check_rvalue(tu, tf, Some(lhs.ty));
                let rhs_class = classify_type(rhs.ty.lookup());

                let (op, ty) = match (lhs_class, op, rhs_class) {
                    // ptr - ptr = int
                    (BinType::Pointer, BinOp::Sub, BinType::Pointer) if lhs.ty == rhs.ty => {
                        (BinOp::Sub, lhs.ty)
                    }
                    // ptr + int = ptr
                    (BinType::Pointer, BinOp::Add, BinType::Integral) => (BinOp::PtrAdd, lhs.ty),
                    // ptr - int = ptr
                    (BinType::Pointer, BinOp::Sub, BinType::Integral) => (BinOp::PtrSub, lhs.ty),
                    // int + ptr = ptr
                    (BinType::Integral, BinOp::Add, BinType::Pointer) => (BinOp::PtrAdd, rhs.ty),
                    // int +,-,/,* int = int
                    (BinType::Integral, op, BinType::Integral)
                        if lhs.ty == rhs.ty && op.is_arithmetic() =>
                    {
                        (*op, lhs.ty)
                    }
                    // int >,>=,<,<= int = bool, ptr >,>=,<,<= ptr = bool
                    (
                        BinType::Integral | BinType::Pointer,
                        op,
                        BinType::Integral | BinType::Pointer,
                    ) if lhs.ty == rhs.ty && op.is_ordered() => (*op, add_type(Type::Bool)),
                    // any == any = bool, any != any = bool
                    (_, BinOp::Eq | BinOp::Ne, _) if lhs.ty == rhs.ty => {
                        (*op, add_type(Type::Bool))
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
                let checked_ty = tu.resolve_type(target_ty.inner);
                println!("Casting to {checked_ty}");
                let checked_rhs = rhs.type_check_rvalue(tu, tf, None);
                let kind = TirExprKind::Cast {
                    target_ty: checked_ty,
                    expr: Box::new(checked_rhs),
                };
                TirExpr::new(kind, checked_ty)
            }
            HirExpr::Call { callee, args } => {
                enum CallKind {
                    Direct,
                    Indirect,
                }

                if let HirExpr::Ident(name) = callee.inner {
                    let Some(info) = lookup_ident(tu, tf, name) else {
                        die!("Use of undefined variable {name}: {span}")
                    };

                    let symbol = info.symbol;

                    if let Type::Function {
                        arg_types,
                        return_type,
                    } = info.ty.lookup()
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
                                arg.type_check_rvalue(tu, tf, Some(*expected_arg_type));
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
                match checked_callee.ty.lookup() {
                    Type::Pointer(f) => {
                        if let Type::Function {
                            arg_types,
                            return_type,
                        } = f.lookup()
                        {
                            let mut checked_args = vec![];
                            let ty = return_type.clone();
                            for (arg, expected_arg_type) in
                                args.iter().zip(arg_types.clone().iter())
                            {
                                let checked_arg =
                                    arg.type_check_rvalue(tu, tf, Some(*expected_arg_type));
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
