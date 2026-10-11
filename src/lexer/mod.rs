use std::{fmt::Display, ops::Range};

use strum::EnumIs;

use crate::{
    core::{Constant, ConstantType, Specifier},
    diagnostics::{Diagnostic, Stage},
};

mod float;
mod hash;

#[derive(Debug, PartialEq, Copy, Clone, EnumIs)]
pub enum Token {
    Ident,
    Int,
    Literal(Constant),
    Long,
    Void,
    Signed,
    Unsigned,
    Return,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Tilde,
    Hyphen,
    Plus,
    Asterisk,
    Slash,
    Percent,
    Ampersand,
    Caret,
    Pipe,
    LeftShift,
    RightShift,
    Not,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    RemainderAssign,
    BitwiseAndAssign,
    BitwiseXorAssign,
    BitwiseOrAssign,
    RightShiftAssign,
    LeftShiftAssign,
    Increment,
    Decrement,
    If,
    Else,
    QuestionMark,
    Colon,
    Goto,
    Do,
    While,
    For,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Comma,
    Static,
    Extern,
    EndOfInput,
    Double,
    Float,
    InfDouble,
    InfFloat,
    NanDouble,
    NanFloat,
}

#[derive(Debug)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
}

pub fn lex(source: &str) -> Result<Vec<SpannedToken>, Vec<Diagnostic>> {
    Lexer::new(source).lex()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<SpannedToken>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: vec![],
            diagnostics: vec![],
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn current(&self) -> u8 {
        self.bytes[self.pos]
    }

    fn double_peek(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.pos += 1;
        }
    }

    fn skip_line(&mut self) {
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == b'\n' {
                break;
            }
        }
    }

    fn push(&mut self, token: SpannedToken) {
        self.tokens.push(token);
    }

    fn lex(mut self) -> Result<Vec<SpannedToken>, Vec<Diagnostic>> {
        while !self.next_token() {}

        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.diagnostics)
        }
    }

    fn next_token(&mut self) -> bool {
        loop {
            self.skip_whitespace();

            if self.peek() == Some(b'#') {
                self.skip_line();
                continue;
            }

            break;
        }

        let start = self.pos;

        let Some(c) = self.peek() else {
            self.push(SpannedToken {
                token: Token::EndOfInput,
                span: self.source.len()..self.source.len(),
            });

            return true;
        };

        match c {
            c if c.is_ascii_alphabetic() || c == b'_' => {
                self.lex_ident_or_keyword(start);
            }
            c if c.is_ascii_digit() => {
                self.lex_number(start);
            }
            b'.' if self.double_peek().is_some_and(|c| c.is_ascii_digit()) => {
                self.lex_number(start)
            }
            _ => self.lex_operator(start),
        }

        false
    }

    fn lex_ident_or_keyword(&mut self, start: usize) {
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }
        let span = start..self.pos;
        let text = &self.source[span.clone()];
        let tokens = keyword_from_str(text).unwrap_or(&[Token::Ident]);

        for token in tokens {
            self.push(SpannedToken {
                token: *token,
                span: span.clone(),
            });
        }
    }

    fn lex_number(&mut self, start: usize) {
        let mut is_float = false;

        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }

        if matches!(self.peek(), Some(c) if c == b'.') {
            is_float = true;
            self.pos += 1;
        }

        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }

        if matches!(self.peek(), Some(c) if c == b'e' || c == b'E') {
            is_float = true;
            self.pos += 1;
        }

        if matches!(self.peek(), Some(c) if c == b'-') {
            is_float = true;
            self.pos += 1;
        }

        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }

        let suffix_start = self.pos;
        // also consuming '.' here catches 0.5.2 otherwise being parsed as [0.5, 0.2], and instead gives an invalid
        // suffix error, which is what clang does
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
        {
            self.pos += 1;
        }

        let span = start..self.pos;
        let text = &self.source[span.clone()];

        // TODO: hex and octal and binary support
        let digits = &self.source[start..suffix_start];
        let candidate_types = match classify_number(
            &self.source[start..suffix_start],
            &self.source[suffix_start..self.pos],
            true,
            is_float,
        ) {
            Ok(constant) => constant,
            Err(e) => {
                self.diagnostics
                    .push(Diagnostic::new(Stage::Lex, span.clone(), e.message(text)));
                return;
            }
        };

        if is_float {
            self.lex_float(start, candidate_types[0]);
        } else {
            self.lex_integer(digits, candidate_types);
        }
    }

    fn lex_integer(&mut self, digits: &str, candidates: &[ConstantType]) {
        // u128 can fit any valid digits for any type, since the negative numbers dont exist at lexing and only at the parsing stage
        let Ok(magnitude) = digits.parse::<u128>() else {
            self.diagnostics.push(Diagnostic::new(
                Stage::Lex,
                self.pos - digits.len()..self.pos,
                format!("integer literal '{digits}' is too large"),
            ));
            return;
        };

        for candidate in candidates {
            if magnitude <= max_value(*candidate) {
                let constant = match candidate {
                    ConstantType::Int => Constant::Int(magnitude as i32),
                    ConstantType::UInt => Constant::UInt(magnitude as u32),
                    ConstantType::Long => Constant::Long(magnitude as i64),
                    ConstantType::ULong => Constant::ULong(magnitude as u64),
                    _ => unreachable!(),
                };

                self.push(SpannedToken {
                    token: Token::Literal(constant),
                    span: self.pos - digits.len()..self.pos,
                });
                return;
            }
        }

        self.diagnostics.push(Diagnostic::new(
            Stage::Lex,
            self.pos - digits.len()..self.pos,
            format!("integer literal '{digits}' is too large for any type"),
        ));
    }

    fn lex_operator(&mut self, start: usize) {
        let three = self.peek_str(3);
        if let Some(tok) = three.and_then(three_char_op) {
            self.pos += 3;
            self.push(SpannedToken {
                token: tok,
                span: start..self.pos,
            });
            return;
        }

        let two = self.peek_str(2);
        if let Some(tok) = two.and_then(two_char_op) {
            self.pos += 2;
            self.push(SpannedToken {
                token: tok,
                span: start..self.pos,
            });
            return;
        }

        let c = self.bump().unwrap();
        if let Some(tok) = one_char_op(c) {
            self.push(SpannedToken {
                token: tok,
                span: start..self.pos,
            });
            return;
        }

        let text: String = (self.source[start..self.pos])
            .chars()
            .map(|c| c.escape_debug().to_string())
            .collect();

        self.diagnostics.push(Diagnostic::new(
            Stage::Lex,
            start..self.pos,
            format!("unexpected character '{text}'"),
        ));
    }

    fn peek_str(&self, len: usize) -> Option<&'a str> {
        let end = self.pos + len;
        if end <= self.bytes.len() {
            self.source.get(self.pos..end)
        } else {
            None
        }
    }
}

fn keyword_from_str(s: &str) -> Option<&[Token]> {
    hash::KEYWORDS.get(s).copied()
}

fn three_char_op(s: &str) -> Option<Token> {
    Some(match s {
        ">>=" => Token::RightShiftAssign,
        "<<=" => Token::LeftShiftAssign,
        _ => return None,
    })
}

fn two_char_op(s: &str) -> Option<Token> {
    hash::TWO_CHAR_OPS.get(s).copied()
}

fn one_char_op(c: u8) -> Option<Token> {
    hash::ONE_CHAR_OPS.get(&c).copied()
}

impl Token {
    pub fn is_specifier(self) -> bool {
        self.specifier().is_some()
    }

    pub fn is_type(self) -> bool {
        matches!(
            self,
            Token::Int
                | Token::Long
                | Token::Unsigned
                | Token::Signed
                | Token::Float
                | Token::Double
        )
    }

    pub fn specifier(self) -> Option<Specifier> {
        match self {
            Token::Int => Some(Specifier::Int),
            Token::Long => Some(Specifier::Long),
            Token::Signed => Some(Specifier::Signed),
            Token::Unsigned => Some(Specifier::Unsigned),
            Token::Static => Some(Specifier::Static),
            Token::Extern => Some(Specifier::Extern),
            Token::Float => Some(Specifier::Float),
            Token::Double => Some(Specifier::Double),
            _ => None,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Token::Literal(c) => match c {
                Constant::Int(_) => "literal_int",
                Constant::Long(_) => "literal_long",
                Constant::UInt(_) => "literal_unsigned_int",
                Constant::ULong(_) => "literal_unsigned_long",
                Constant::Float(_) => "literal_float",
                Constant::Double(_) => "literal_double",
            },
            Token::Ident => "identifier",
            Token::Int => "int",
            Token::Long => "long",
            Token::Void => "void",
            Token::Signed => "signed",
            Token::Unsigned => "unsigned",
            Token::Double => "double",
            Token::Float => "float",
            Token::Return => "return",
            Token::OpenParen => "(",
            Token::CloseParen => ")",
            Token::OpenBrace => "{",
            Token::CloseBrace => "}",
            Token::Semicolon => ";",
            Token::Tilde => "~",
            Token::Hyphen => "-",
            Token::Plus => "+",
            Token::Asterisk => "*",
            Token::Slash => "/",
            Token::Percent => "%",
            Token::Ampersand => "&",
            Token::Caret => "^",
            Token::Pipe => "|",
            Token::LeftShift => "<<",
            Token::RightShift => ">>",
            Token::Not => "!",
            Token::LogicalAnd => "&&",
            Token::LogicalOr => "||",
            Token::Equal => "==",
            Token::NotEqual => "!=",
            Token::LessThan => "<",
            Token::LessEqual => "<=",
            Token::GreaterThan => ">",
            Token::GreaterEqual => ">=",
            Token::Assign => "=",
            Token::AddAssign => "+=",
            Token::SubtractAssign => "-=",
            Token::MultiplyAssign => "*=",
            Token::DivideAssign => "/=",
            Token::RemainderAssign => "%=",
            Token::BitwiseAndAssign => "&=",
            Token::BitwiseXorAssign => "^=",
            Token::BitwiseOrAssign => "|=",
            Token::RightShiftAssign => ">>=",
            Token::LeftShiftAssign => "<<=",
            Token::Increment => "++",
            Token::Decrement => "--",
            Token::If => "if",
            Token::Else => "else",
            Token::QuestionMark => "?",
            Token::Colon => ":",
            Token::Goto => "goto",
            Token::EndOfInput => "EOF",
            Token::Do => "do",
            Token::While => "while",
            Token::For => "for",
            Token::Switch => "switch",
            Token::Case => "case",
            Token::Default => "default",
            Token::Break => "break",
            Token::Continue => "continue",
            Token::Comma => ",",
            Token::Static => "static",
            Token::Extern => "extern",
            Token::NanDouble => "nan_double",
            Token::NanFloat => "nan_float",
            Token::InfDouble => "inf_double",
            Token::InfFloat => "inf_float",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumSuffix {
    None,
    Long,
    Unsigned,
    UnsignedLong,
    Float,
    Double,
}

impl NumSuffix {
    fn parse(suffix: &str) -> Result<Self, NumLiteralError> {
        match suffix {
            // rust like
            "u32" => return Ok(NumSuffix::Unsigned),
            "u64" => return Ok(NumSuffix::UnsignedLong),
            "i32" => return Ok(NumSuffix::None),
            "i64" => return Ok(NumSuffix::Long),
            "f32" => return Ok(NumSuffix::Float),
            "f64" => return Ok(NumSuffix::Double),
            // c like (probably will remove once done)
            _ => {
                let mut has_l = false;
                let mut has_u = false;
                for c in suffix.chars() {
                    match c.to_ascii_lowercase() {
                        'l' if !has_l => has_l = true,
                        'u' if !has_u => has_u = true,
                        _ => return Err(NumLiteralError::BadSuffix(suffix.to_string())),
                    }
                }
                Ok(match (has_l, has_u) {
                    (false, false) => NumSuffix::None,
                    (true, false) => NumSuffix::Long,
                    (false, true) => NumSuffix::Unsigned,
                    (true, true) => NumSuffix::UnsignedLong,
                })
            }
        }
    }
}

fn constant_type_candidates(
    suffix: NumSuffix,
    decimal: bool,
    fractional: bool,
) -> Result<&'static [ConstantType], NumLiteralError> {
    use ConstantType::*;

    if fractional {
        match suffix {
            NumSuffix::Float => Ok(&[Float]),
            NumSuffix::None | NumSuffix::Double => Ok(&[Double]),
            _ => Err(NumLiteralError::InvalidFloatSuffix),
        }
    } else {
        match (suffix, decimal) {
            (NumSuffix::None, true) => Ok(&[Int, Long]),
            (NumSuffix::None, false) => Ok(&[Int, UInt, Long, ULong]),
            (NumSuffix::Unsigned, _) => Ok(&[UInt, ULong]),
            (NumSuffix::Long, _) => Ok(&[Long]),
            (NumSuffix::UnsignedLong, _) => Ok(&[ULong]),
            (NumSuffix::Float | NumSuffix::Double, _) => Err(NumLiteralError::InvalidIntSuffix),
        }
    }
}

const fn max_value(ty: ConstantType) -> u128 {
    match ty {
        ConstantType::Int => i32::MAX as u128,
        ConstantType::UInt => u32::MAX as u128,
        ConstantType::Long => i64::MAX as u128,
        ConstantType::ULong => u64::MAX as u128,
        ConstantType::Float | ConstantType::Double => unreachable!(),
    }
}

#[derive(Debug)]
enum NumLiteralError {
    BadSuffix(String),
    InvalidFloatSuffix,
    InvalidIntSuffix,
}

impl NumLiteralError {
    fn message(&self, literal: &str) -> String {
        match self {
            Self::BadSuffix(suffix) => {
                format!("invalid suffix '{suffix}' on constant '{literal}'")
            }
            Self::InvalidFloatSuffix => {
                format!("invalid suffix on floating point constant '{literal}'")
            }
            Self::InvalidIntSuffix => format!("invalid suffix on integer constant '{literal}'"),
        }
    }
}

fn classify_number(
    digits: &str,
    suffix: &str,
    decimal: bool,
    fractional: bool,
) -> Result<&'static [ConstantType], NumLiteralError> {
    let suffix = NumSuffix::parse(suffix)?;
    constant_type_candidates(suffix, decimal, fractional)
}
