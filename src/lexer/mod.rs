use std::{fmt::Display, ops::Range};

use strum::EnumIs;

use crate::{
    diagnostics::{Diagnostic, Stage},
    parser::{Constant, ConstantType, Specifier},
};

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
}

#[derive(Debug)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
}

pub fn lex(source: &str) -> Result<Vec<SpannedToken>, Vec<Diagnostic>> {
    let mut lexer = Lexer::new(source);
    let mut tokens = vec![];
    let mut errors = vec![];

    loop {
        match lexer.next_token() {
            Ok(spanned) => {
                let is_eof = spanned.token.is_end_of_input();
                tokens.push(spanned);
                if is_eof {
                    break;
                }
            }
            Err(diag) => errors.push(diag),
        }
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
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

    fn next_token(&mut self) -> Result<SpannedToken, Diagnostic> {
        self.skip_whitespace();

        let start = self.pos;

        let Some(c) = self.peek() else {
            return Ok(SpannedToken {
                token: Token::EndOfInput,
                span: self.source.len()..self.source.len(),
            });
        };

        if c.is_ascii_alphabetic() || c == b'_' {
            return Ok(self.lex_ident_or_keyword(start));
        }

        if c.is_ascii_digit() {
            return self.lex_number(start);
        }

        self.lex_operator(start)
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> SpannedToken {
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }
        let span = start..self.pos;
        let text = &self.source[span.clone()];
        let token = keyword_from_str(text).unwrap_or(Token::Ident);
        SpannedToken { token, span }
    }

    fn lex_number(&mut self, start: usize) -> Result<SpannedToken, Diagnostic> {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let digits_end = self.pos;

        let suffix_start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }

        let span = start..self.pos;
        let text = &self.source[span.clone()];

        // TODO: hex and octal and binary support arent done yet. i'll wait to see if they come up in the book
        let constant = classify_integer(
            &self.source[start..digits_end],
            &self.source[suffix_start..self.pos],
            true,
        )
        .map_err(|e| Diagnostic::new(Stage::Lex, span.clone(), e.message(text)))?;

        Ok(SpannedToken {
            token: Token::Literal(constant),
            span,
        })
    }

    fn lex_operator(&mut self, start: usize) -> Result<SpannedToken, Diagnostic> {
        let three = self.peek_str(3);
        if let Some(tok) = three.and_then(three_char_op) {
            self.pos += 3;
            return Ok(SpannedToken {
                token: tok,
                span: start..self.pos,
            });
        }

        let two = self.peek_str(2);
        if let Some(tok) = two.and_then(two_char_op) {
            self.pos += 2;
            return Ok(SpannedToken {
                token: tok,
                span: start..self.pos,
            });
        }

        let c = self.bump().unwrap();
        if let Some(tok) = one_char_op(c) {
            return Ok(SpannedToken {
                token: tok,
                span: start..self.pos,
            });
        }

        let text: String = (self.source[start..self.pos])
            .chars()
            .map(|c| c.escape_debug().to_string())
            .collect();
        Err(Diagnostic::new(
            Stage::Lex,
            start..self.pos,
            format!("unexpected character '{text}'"),
        ))
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

fn keyword_from_str(s: &str) -> Option<Token> {
    Some(match s {
        "int" => Token::Int,
        "long" => Token::Long,
        "void" => Token::Void,
        "signed" => Token::Signed,
        "unsigned" => Token::Unsigned,
        "return" => Token::Return,
        "if" => Token::If,
        "else" => Token::Else,
        "goto" => Token::Goto,
        "do" => Token::Do,
        "while" => Token::While,
        "for" => Token::For,
        "switch" => Token::Switch,
        "case" => Token::Case,
        "default" => Token::Default,
        "break" => Token::Break,
        "continue" => Token::Continue,
        "static" => Token::Static,
        "extern" => Token::Extern,
        _ => return None,
    })
}

fn three_char_op(s: &str) -> Option<Token> {
    Some(match s {
        ">>=" => Token::RightShiftAssign,
        "<<=" => Token::LeftShiftAssign,
        _ => return None,
    })
}

fn two_char_op(s: &str) -> Option<Token> {
    Some(match s {
        "<<" => Token::LeftShift,
        ">>" => Token::RightShift,
        "&&" => Token::LogicalAnd,
        "||" => Token::LogicalOr,
        "==" => Token::Equal,
        "!=" => Token::NotEqual,
        "<=" => Token::LessEqual,
        ">=" => Token::GreaterEqual,
        "+=" => Token::AddAssign,
        "-=" => Token::SubtractAssign,
        "*=" => Token::MultiplyAssign,
        "/=" => Token::DivideAssign,
        "%=" => Token::RemainderAssign,
        "&=" => Token::BitwiseAndAssign,
        "^=" => Token::BitwiseXorAssign,
        "|=" => Token::BitwiseOrAssign,
        "++" => Token::Increment,
        "--" => Token::Decrement,
        _ => return None,
    })
}

fn one_char_op(c: u8) -> Option<Token> {
    Some(match c {
        b'(' => Token::OpenParen,
        b')' => Token::CloseParen,
        b'{' => Token::OpenBrace,
        b'}' => Token::CloseBrace,
        b';' => Token::Semicolon,
        b'~' => Token::Tilde,
        b'-' => Token::Hyphen,
        b'+' => Token::Plus,
        b'*' => Token::Asterisk,
        b'/' => Token::Slash,
        b'%' => Token::Percent,
        b'&' => Token::Ampersand,
        b'^' => Token::Caret,
        b'|' => Token::Pipe,
        b'!' => Token::Not,
        b'<' => Token::LessThan,
        b'>' => Token::GreaterThan,
        b'=' => Token::Assign,
        b'?' => Token::QuestionMark,
        b':' => Token::Colon,
        b',' => Token::Comma,
        _ => return None,
    })
}

impl Token {
    pub fn is_specifier(self) -> bool {
        self.specifier().is_some()
    }

    pub fn is_type(self) -> bool {
        matches!(
            self,
            Token::Int | Token::Long | Token::Unsigned | Token::Signed
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
            _ => None,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Token::Ident => "identifier",
            Token::Literal(c) => match c {
                Constant::Int(_) => "literal_int",
                Constant::Long(_) => "literal_long",
                Constant::UInt(_) => "literal_unsigned_int",
                Constant::ULong(_) => "literal_unsigned_long",
            },
            Token::Int => "int",
            Token::Long => "long",
            Token::Void => "void",
            Token::Signed => "signed",
            Token::Unsigned => "unsigned",
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
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntSuffix {
    None,
    Long,
    Unsigned,
    UnsignedLong,
}

impl IntSuffix {
    fn parse(suffix: &str) -> Result<Self, IntLiteralError> {
        let mut has_l = false;
        let mut has_u = false;
        for c in suffix.chars() {
            match c.to_ascii_lowercase() {
                'l' if !has_l => has_l = true,
                'u' if !has_u => has_u = true,
                _ => return Err(IntLiteralError::BadSuffix(suffix.to_string())),
            }
        }
        Ok(match (has_l, has_u) {
            (false, false) => IntSuffix::None,
            (true, false) => IntSuffix::Long,
            (false, true) => IntSuffix::Unsigned,
            (true, true) => IntSuffix::UnsignedLong,
        })
    }
}

fn constant_type_candidates(suffix: IntSuffix, decimal: bool) -> &'static [ConstantType] {
    use ConstantType::*;
    match (suffix, decimal) {
        (IntSuffix::None, true) => &[Int, Long],
        (IntSuffix::None, false) => &[Int, UInt, Long, ULong],
        (IntSuffix::Unsigned, _) => &[UInt, ULong],
        (IntSuffix::Long, _) => &[Long],
        (IntSuffix::UnsignedLong, _) => &[ULong],
    }
}

const fn max_value(ty: ConstantType) -> u128 {
    match ty {
        ConstantType::Int => i32::MAX as u128,
        ConstantType::UInt => u32::MAX as u128,
        ConstantType::Long => i64::MAX as u128,
        ConstantType::ULong => u64::MAX as u128,
    }
}

#[derive(Debug)]
enum IntLiteralError {
    BadSuffix(String),
    TooLarge,
}

impl IntLiteralError {
    fn message(&self, literal: &str) -> String {
        match self {
            Self::BadSuffix(suffix) => {
                format!("invalid suffix '{suffix}' on integer constant '{literal}'")
            }
            Self::TooLarge => {
                format!("integer constant '{literal}' is too large for any integer type")
            }
        }
    }
}

fn classify_integer(
    digits: &str,
    suffix: &str,
    decimal: bool,
) -> Result<Constant, IntLiteralError> {
    let suffix = IntSuffix::parse(suffix)?;

    // u128 holds anything that fits in any type we support, so a parse
    // failure means the value is out of range for every candidate.
    // remember that negatives are lexed as a minus sign and then a positive literal
    let magnitude: u128 = digits.parse().map_err(|_| IntLiteralError::TooLarge)?;

    constant_type_candidates(suffix, decimal)
        .iter()
        .copied()
        .find(|&ty| magnitude <= max_value(ty))
        .map(|ty| Constant::from_magnitude(magnitude, ty))
        .ok_or(IntLiteralError::TooLarge)
}
