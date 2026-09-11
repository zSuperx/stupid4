use super::lexer::Lexer;
use super::token::Token;

use crate::IRs::hir::*;
use crate::ast::*;
use crate::common::{Span, Spanned};
use crate::die;
use crate::translation_unit::add_type;

pub struct Parser {
    lexer: Lexer,
}

pub fn parse_file(filename: &str) -> Vec<Spanned<HirObj>> {
    let mut p = Parser::new(filename);
    let mut objects = vec![];
    while p.lexer.peek().inner != Token::Eof {
        objects.push(p.parse_obj())
    }
    objects
}

impl Parser {
    pub fn new(filename: &str) -> Parser {
        let filename: &'static str = filename.to_string().leak();
        Self {
            lexer: Lexer::new(filename),
        }
    }

    /// Start a span. This does not change state, but simply returns
    /// the span of the next token that will be consumed.
    /// Use this before you start parsing a "thing", i.e...
    ///
    /// let start = self.mark();
    /// self.expect(Token::Fn);
    /// self.expect(...);
    /// self.commit(..., start)
    fn mark(&mut self) -> Span {
        self.lexer.peek().span
    }

    /// Creates a new Spanned "thing", given a starting point
    fn commit<T>(&mut self, inner: T, start: Span) -> Spanned<T> {
        Spanned::new(inner, start.merge(self.lexer.last_span))
    }

    pub fn parse_obj(&mut self) -> Spanned<HirObj> {
        let tok = self.lexer.peek();
        match tok.inner {
            Token::Fn => self.parse_func(),
            Token::Global => self.parse_global(),
            Token::Struct => self.parse_struct(),
            _ => die!("Expected `global`, `fn`, or `struct`, but got {tok}"),
        }
    }

    fn parse_struct(&mut self) -> Spanned<HirObj> {
        let span_start = self.mark();
        self.lexer.expect(Token::Struct);
        let name = self.lexer.expect_ident();
        self.lexer.expect(Token::LCurly);
        let mut fields = vec![];
        while !self.lexer.is_next(Token::RCurly) {
            let field_name = self.lexer.expect_ident();
            self.lexer.expect(Token::Colon);
            let field_ty = self.parse_type();
            self.lexer.expect(Token::Semi);
            fields.push((field_name, field_ty));
        }
        self.lexer.expect(Token::RCurly);
        let inner = HirObj::Struct { name, fields };
        self.commit(inner, span_start)
    }

    fn parse_global(&mut self) -> Spanned<HirObj> {
        let span_start = self.mark();
        self.lexer.expect(Token::Global);
        let name = self.lexer.expect_ident();
        self.lexer.expect(Token::Colon);
        let ty = self.parse_type();
        self.lexer.expect(Token::Eq);
        let rhs = self.parse_expr();
        self.lexer.expect(Token::Semi);
        let global = HirObj::Global {
            name,
            ty,
            rhs: Box::new(rhs),
        };
        self.commit(global, span_start)
    }

    fn parse_func(&mut self) -> Spanned<HirObj> {
        let span_start = self.mark();
        self.lexer.expect(Token::Fn);
        let name = self.lexer.expect_ident();

        self.lexer.expect(Token::LParen);
        let mut args = vec![];
        while self.lexer.peek().inner != Token::RParen {
            if !args.is_empty() {
                self.lexer.expect(Token::Comma);
                if self.lexer.is_next(Token::RParen) {
                    break;
                }
            }
            let argname = self.lexer.expect_ident();
            self.lexer.expect(Token::Colon);
            let ty = self.parse_type();
            args.push((argname, ty));
        }
        self.lexer.expect(Token::RParen);

        let peeked = self.lexer.peek();
        let returns = match peeked.inner {
            Token::Colon => {
                self.lexer.expect(Token::Colon);
                self.parse_type()
            }
            Token::LCurly => {
                let span_start = self.mark();
                let ty = Type::Void;
                let id = add_type(ty);
                self.commit(id, span_start)
            }
            _ => die!("Expected return type or function body, found {peeked}"),
        };

        let body = self.parse_block();

        let obj = HirObj::Fn(HirFunction {
            name,
            return_type: returns,
            args,
            body,
        });
        self.commit(obj, span_start)
    }

    fn parse_type(&mut self) -> Spanned<TypeId> {
        let span_start = self.mark();
        let tok = self.lexer.peek();
        let ty = match tok.inner {
            Token::Star => {
                self.lexer.expect(Token::Star);
                Type::Pointer(self.parse_type().inner)
            }
            _ => Type::Unresolved(self.lexer.expect_ident().inner),
        };

        let id = add_type(ty);

        self.commit(id, span_start)
    }

    fn parse_stmt(&mut self) -> Spanned<HirStmt> {
        let span_start = self.mark();
        let tok = self.lexer.peek();
        let stmt = match tok.inner {
            Token::Let => {
                self.lexer.eat();
                let lhs = self.lexer.expect_ident();
                let ty = self.lexer.is_next(Token::Colon).then(|| {
                    self.lexer.expect(Token::Colon);
                    self.parse_type()
                });
                self.lexer.expect(Token::Eq);
                let rhs = self.parse_expr();
                self.lexer.expect(Token::Semi);
                HirStmt::Let { lhs, ty, rhs }
            }
            Token::If => {
                self.lexer.eat();
                let cond = self.parse_expr();
                let then_ = self.parse_block();
                let else_ = if self.lexer.is_next(Token::Else) {
                    self.lexer.eat();
                    if self.lexer.is_next(Token::If) {
                        self.parse_stmt()
                    } else {
                        self.parse_block()
                    }
                } else {
                    let stmt = HirStmt::Block(vec![]);
                    Spanned::new(stmt, Span::default())
                };

                HirStmt::If {
                    cond,
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            }
            Token::Break => {
                self.lexer.eat();
                self.lexer.expect(Token::Semi);
                HirStmt::Break
            }
            Token::Continue => {
                self.lexer.eat();
                self.lexer.expect(Token::Semi);
                HirStmt::Continue
            }
            Token::Return => {
                self.lexer.eat();
                let ret_val = if self.lexer.is_next(Token::Semi) {
                    None
                } else {
                    Some(self.parse_expr())
                };
                let stmt = HirStmt::Return(ret_val);
                self.lexer.expect(Token::Semi);
                stmt
            }
            Token::While => {
                self.lexer.eat();
                let cond = self.parse_expr();
                let body = self.parse_block();
                HirStmt::While {
                    cond,
                    body: Box::new(body),
                }
            }
            _ => {
                let expr = self.parse_expr();
                self.lexer.expect(Token::Semi);
                HirStmt::Expr(expr)
            }
        };
        self.commit(stmt, span_start)
    }

    fn parse_block(&mut self) -> Spanned<HirStmt> {
        let span_start = self.mark();
        self.lexer.expect(Token::LCurly);
        let mut stmts = vec![];
        while !self.lexer.is_next(Token::RCurly) {
            let stmt = self.parse_stmt();
            if matches!(stmt.inner, HirStmt::Block(ref inner) if inner.is_empty()) {
                // Skip nested empty {} blocks, they're useless.
                continue;
            }
            stmts.push(stmt);
        }
        self.lexer.expect(Token::RCurly);
        let stmt = HirStmt::Block(stmts);
        self.commit(stmt, span_start)
    }

    fn parse_prefix(&mut self) -> Spanned<HirExpr> {
        let span_start = self.mark();
        let tok = self.lexer.eat();
        let typed_expr = match tok.inner {
            Token::Sizeof => {
                if self.lexer.is_next(Token::At) {
                    self.lexer.expect(Token::At);
                    self.lexer.expect(Token::LParen);
                    let ty = self.parse_type();
                    self.lexer.expect(Token::RParen);
                    HirExpr::SizeOfTy { ty }
                } else {
                    self.lexer.expect(Token::LParen);
                    let expr = self.parse_expr();
                    self.lexer.expect(Token::RParen);
                    HirExpr::SizeOfExpr {
                        expr: Box::new(expr),
                    }
                }
            }
            Token::Bang => {
                let rhs = Box::new(self.parse_expr());
                HirExpr::Un { op: UnOp::Not, rhs }
            }
            Token::Ident(s) => HirExpr::Ident(s),
            Token::Minus => {
                let rhs = Box::new(self.parse_expr());
                HirExpr::Un { op: UnOp::Neg, rhs }
            }
            Token::Bool(x) => HirExpr::Bool(x),
            Token::Int(x) => HirExpr::Num(x),
            Token::LParen => {
                let inner_expr = self.parse_expr();
                self.lexer.expect(Token::RParen);
                inner_expr.inner
            }
            Token::Star => {
                let power = prefix_power(Token::Star).unwrap();
                let rhs = Box::new(self._parse_expr(power));
                HirExpr::Deref { inner: rhs }
            }
            Token::And => {
                let power = prefix_power(Token::Star).unwrap();
                let rhs = Box::new(self._parse_expr(power));
                HirExpr::AddrOf { inner: rhs }
            }
            Token::At => {
                let power = prefix_power(Token::At).unwrap();
                self.lexer.expect(Token::LParen);
                let target_ty = self.parse_type();
                self.lexer.expect(Token::RParen);
                let rhs = Box::new(self._parse_expr(power));
                HirExpr::Cast { target_ty, rhs }
            }
            _ => die!("Expected start of expression, found {tok}"),
        };
        self.commit(typed_expr, span_start.merge(self.lexer.last_span))
    }

    fn parse_infix(
        &mut self,
        lhs: Spanned<HirExpr>,
        op: Spanned<Token>,
        op_power: f32,
    ) -> Spanned<HirExpr> {
        let span_start = lhs.span;
        let output = match op.inner {
            Token::LBrack => {
                let idx = self.parse_expr();
                self.lexer.expect(Token::RBrack);
                HirExpr::Index {
                    base: Box::new(lhs),
                    index: Box::new(idx),
                }
            }
            Token::Dot => {
                let field_name = self.lexer.expect_ident();
                HirExpr::Field {
                    base: Box::new(lhs),
                    field: field_name.inner,
                }
            }
            Token::LParen => {
                let mut args = vec![];
                while !self.lexer.is_next(Token::RParen) {
                    if !args.is_empty() {
                        self.lexer.expect(Token::Comma);
                    }
                    args.push(self.parse_expr());
                }
                self.lexer.expect(Token::RParen);
                HirExpr::Call {
                    callee: Box::new(lhs),
                    args,
                }
            }
            Token::Eq => {
                let rhs = self._parse_expr(op_power);
                HirExpr::Assign {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            arith @ (Token::Plus | Token::Minus | Token::Star | Token::Slash) => {
                let rhs = self._parse_expr(op_power);
                let op = match arith {
                    Token::Plus => BinOp::Add,
                    Token::Minus => BinOp::Sub,
                    Token::Star => BinOp::Mul,
                    Token::Slash => BinOp::Div,
                    _ => unreachable!(),
                };

                HirExpr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            rel @ (Token::EqEq
            | Token::BangEq
            | Token::LtEq
            | Token::Lt
            | Token::GtEq
            | Token::Gt) => {
                let rhs = self._parse_expr(op_power);
                let op = match rel {
                    Token::BangEq => BinOp::Ne,
                    Token::EqEq => BinOp::Eq,
                    Token::LtEq => BinOp::Le,
                    Token::Lt => BinOp::Lt,
                    Token::GtEq => BinOp::Ge,
                    Token::Gt => BinOp::Gt,
                    _ => unreachable!(),
                };

                HirExpr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            _ => die!("Expected infix operator, found {op}"),
        };
        self.commit(output, span_start)
    }

    // Pratt parsing!
    fn _parse_expr(&mut self, min_power: f32) -> Spanned<HirExpr> {
        let mut lhs = self.parse_prefix();

        loop {
            let op = self.lexer.peek();

            let Some(op_power) = infix_power(op.inner) else {
                break;
            };

            if op_power.0 < min_power {
                break;
            }

            self.lexer.eat();

            lhs = self.parse_infix(lhs, op, op_power.1);
        }

        lhs
    }

    fn parse_expr(&mut self) -> Spanned<HirExpr> {
        self._parse_expr(0.0)
    }
}

fn prefix_power(kind: Token) -> Option<f32> {
    let power = match kind {
        Token::Sizeof | Token::Bang | Token::Minus | Token::Star | Token::And | Token::At => 9.0,
        _ => return None, // Not an prefix operator
    };
    Some(power)
}

fn infix_power(kind: Token) -> Option<(f32, f32)> {
    let power = match kind {
        // Assignment: Right-Associative
        // We use a lower Left power so it "gives up" easily,
        // but a higher Right power to "grab" everything to the right.
        Token::Eq => (2.1, 2.0),

        Token::AndAnd | Token::OrOr => (4.1, 4.0),

        Token::EqEq | Token::BangEq => (5.0, 5.1),
        Token::Lt | Token::Gt => (6.0, 6.1),
        Token::LtEq | Token::GtEq => (6.0, 6.1),

        Token::Plus | Token::Minus => (7.0, 7.1),

        Token::Star | Token::Slash => (8.0, 8.1),

        // Postfix: Highest priority
        // (Call, Indexing, Member Access)
        Token::LParen | Token::LBrack | Token::Dot => (10.0, 10.1),

        _ => return None, // Not an infix operator
    };
    Some(power)
}
