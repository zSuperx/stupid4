use crate::common::RcString;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Token {
    #[default]
    Eof,
    Int(RcString),
    Bool(bool),
    Str(RcString),
    Ident(RcString),

    // Declarator keywords
    Let,
    Fn,
    Struct,
    Global,

    // Control flow keywords
    While,
    Continue,
    Break,
    If,
    Else,
    Return,

    // Delimiters
    LParen, // (
    RParen, // )
    LCurly, // {
    RCurly, // }
    LBrack, // [
    RBrack, // ]

    // Separators
    Comma,  // ,
    Dot,    // .
    Colon,  // :
    Semi,   // ;
    RArrow, // ->

    // Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    And,     // &
    Or,      // |
    Caret,   // ^
    Bang,    // !
    Eq,      // =
    AndAnd,  // &&
    OrOr,    // ||
    At,      // @

    // Relationals
    EqEq,   // ==
    BangEq, // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=

    Sizeof, // sizeof
}
