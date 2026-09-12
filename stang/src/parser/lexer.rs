use super::token::Token;
use crate::common::{Span, Spanned, RcString};
use crate::die;
use crate::translation_unit::{SOURCE, add_str, source};

pub(super) struct Lexer {
    filename: &'static str,
    cursor: usize,
    row: usize,
    col: usize,
    pub(super) last_span: Span,
}

impl Lexer {
    pub fn new(filename: &'static str) -> Self {
        let Ok(source) = std::fs::read(filename) else {
            die!("Could not open file: {filename}");
        };
        SOURCE.set(source).unwrap();
        Self {
            filename,
            row: 0,
            col: 0,
            cursor: 0,
            last_span: Span::new(filename, 0, 0, 0, 0),
        }
    }

    pub fn peek(&mut self) -> Spanned<Token> {
        // Screenshot state
        let tmp = (self.cursor, self.row, self.col);
        // Eat a token
        let tok = self.eat();
        // Restore state
        (self.cursor, self.row, self.col) = tmp;
        // Return the peeked token
        tok
    }

    pub fn eat(&mut self) -> Spanned<Token> {
        let tok = self
            .read_whitespace()
            .or_else(|| self.read_word())
            .or_else(|| self.read_num())
            .or_else(|| self.read_strlit())
            .or_else(|| self.read_punct())
            .unwrap_or_default();
        self.last_span = tok.span;
        tok
    }

    pub fn expect(&mut self, expected: Token) {
        let next = self.eat();
        if next.inner != expected {
            next.span.content();
            die!("Expected {expected:?}, found {next}");
        }
    }

    pub fn is_next(&mut self, expected: Token) -> bool {
        self.peek().inner == expected
    }

    pub fn expect_ident(&mut self) -> Spanned<RcString> {
        let next = self.eat();
        match next.inner {
            Token::Ident(s) => next.span.wrap(s),
            _ => die!("Expected identifier, found {next}"),
        }
    }

    fn peek_byte(&mut self) -> Option<u8> {
        source().get(self.cursor).copied()
    }

    fn eat_byte(&mut self) -> Option<u8> {
        self.cursor += 1;
        match source().get(self.cursor - 1).copied() {
            Some(c) => {
                if c == b'\n' {
                    self.row += 1;
                    self.col = 0;
                } else {
                    self.col += 1;
                }
                Some(c)
            }
            None => None,
        }
    }

    fn make_token(&mut self, kind: Token, lo: usize) -> Spanned<Token> {
        Spanned::new(
            kind,
            Span::new(self.filename, lo, self.cursor, self.row, self.col),
        )
    }

    fn read_num(&mut self) -> Option<Spanned<Token>> {
        let start = self.cursor;
        let mut buf = vec![];

        // Make sure the number starts with a digit
        let first = self.peek_byte()?;
        if !first.is_ascii_digit() {
            return None;
        }

        // Now read all digits and underscores
        while let Some(c) = self.peek_byte()
            && b"0123456789_".contains(&c)
        {
            let c = self.eat_byte()?;
            if c != b'_' {
                buf.push(c);
            }
        }

        let Ok(raw) = String::from_utf8(buf) else {
            die!("Non-utf8 characters are not supported: {}", self.last_span);
        };

        let kind = Token::Int(add_str(&raw));

        Some(self.make_token(kind, start))
    }

    // This returns either an Ident or a Keyword, depending on what the string equates to
    fn read_word(&mut self) -> Option<Spanned<Token>> {
        let start = self.cursor;
        // Identifiers can only start with letters or underscores
        let first = self.peek_byte()?;
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return None;
        }

        while let Some(c) = self.peek_byte()
            && (c.is_ascii_alphanumeric() || c == b'_')
        {
            self.eat_byte()?;
        }

        let Ok(raw) = str::from_utf8(source().get(start..self.cursor)?) else {
            die!("Non-utf8 characters are not supported: {}", self.last_span);
        };

        let kind = match raw {
            "let" => Token::Let,
            "fn" => Token::Fn,
            "struct" => Token::Struct,
            "global" => Token::Global,
            "while" => Token::While,
            "continue" => Token::Continue,
            "break" => Token::Break,
            "if" => Token::If,
            "else" => Token::Else,
            "return" => Token::Return,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "sizeof" => Token::Sizeof,
            _ => Token::Ident(add_str(&raw.to_string())),
        };

        Some(self.make_token(kind, start))
    }

    fn read_strlit(&mut self) -> Option<Spanned<Token>> {
        let start = self.cursor;
        if self.peek_byte()? == b'"' {
            self.eat_byte();
        } else {
            return None;
        }

        loop {
            let curr = self.eat_byte().expect("LEXER: Unclosed quote");
            match curr {
                b'"' => break,
                b'\\' => {
                    self.eat_byte().expect("LEXER: Unclosed quote");
                }
                _ => {}
            }
        }

        // +1/-1 to disclude the surrounding "..."
        let Ok(raw) = str::from_utf8(source().get(start + 1..self.cursor - 1)?) else {
            die!("Non-utf8 characters are not supported: {}", self.last_span);
        };
        let token = Token::Str(add_str(&raw.to_string()));

        Some(self.make_token(token, start))
    }

    fn read_punct(&mut self) -> Option<Spanned<Token>> {
        let start = self.cursor;
        let known_punctuators = &["==", "!=", "<=", ">=", "->", "&&", "||", "<<", ">>"];

        let mut length = 1;
        let src = source().get(start..)?;
        for p in known_punctuators {
            if src.starts_with(p.as_bytes()) {
                length = p.len();
                break;
            }
        }

        let first = self.peek_byte()?;

        if length == 1 && !first.is_ascii_punctuation() || first == b'_' {
            return None;
        }

        let s = (0..length)
            .filter_map(|_| self.eat_byte())
            .collect::<Vec<_>>();

        use Token::*;
        let kind = match s.as_slice() {
            // Delimiters
            b"(" => LParen,
            b")" => RParen,
            b"{" => LCurly,
            b"}" => RCurly,
            b"[" => LBrack,
            b"]" => RBrack,
            // Separators
            b"," => Comma,
            b"." => Dot,
            b":" => Colon,
            b";" => Semi,
            b"->" => RArrow,
            // Operators
            b"+" => Plus,    // +
            b"-" => Minus,   // -
            b"*" => Star,    // *
            b"/" => Slash,   // /
            b"%" => Percent, // %
            b"&" => And,     // &
            b"|" => Or,      // |
            b"^" => Caret,   // ^
            b"!" => Bang,    // !
            b"=" => Eq,      // =
            b"&&" => AndAnd,
            b"||" => OrOr,
            // Relationals
            b"==" => EqEq,
            b"!=" => BangEq,
            b"<" => Lt,
            b">" => Gt,
            b"<=" => LtEq,
            b">=" => GtEq,
            b"@" => At,
            x => die!(
                "Unknown token: `{}` found near: {}",
                str::from_utf8(x).unwrap(),
                self.last_span
            ),
        };

        Some(self.make_token(kind, start))
    }

    fn read_whitespace(&mut self) -> Option<Spanned<Token>> {
        while let Some(c) = self.peek_byte()
            && c.is_ascii_whitespace()
        {
            self.eat_byte()?;
        }
        None
    }
}
