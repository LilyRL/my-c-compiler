use std::{fmt::Display, ops::Range};

use logos::Logos;
use strum::EnumIs;

use crate::{
    diagnostics::{Diagnostic, Stage},
    parser::Specifier,
};

#[derive(Logos, Debug, PartialEq, Copy, Clone, EnumIs)]
#[logos(skip r"[ \t\r\n]+")]
pub enum Token {
    #[regex(r"[a-zA-Z_]\w*")]
    Ident,
    #[regex("[0-9]+")]
    ConstantInt,
    #[token("int")]
    Int,
    #[token("void")]
    Void,
    #[token("return")]
    Return,
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token("{")]
    OpenBrace,
    #[token("}")]
    CloseBrace,
    #[token(";")]
    Semicolon,
    #[token("~")]
    Tilde,
    #[token("-")]
    Hyphen,
    #[token("+")]
    Plus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Ampersand,
    #[token("^")]
    Caret,
    #[token("|")]
    Pipe,
    #[token("<<")]
    LeftShift,
    #[token(">>")]
    RightShift,
    #[token("!")]
    Not,
    #[token("&&")]
    LogicalAnd,
    #[token("||")]
    LogicalOr,
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token("<")]
    LessThan,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    GreaterThan,
    #[token(">=")]
    GreaterEqual,
    #[token("=")]
    Assign,
    #[token("+=")]
    AddAssign,
    #[token("-=")]
    SubtractAssign,
    #[token("*=")]
    MultiplyAssign,
    #[token("/=")]
    DivideAssign,
    #[token("%=")]
    RemainderAssign,
    #[token("&=")]
    BitwiseAndAssign,
    #[token("^=")]
    BitwiseXorAssign,
    #[token("|=")]
    BitwiseOrAssign,
    #[token(">>=")]
    RightShiftAssign,
    #[token("<<=")]
    LeftShiftAssign,
    #[token("++")]
    Increment,
    #[token("--")]
    Decrement,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("?")]
    QuestionMark,
    #[token(":")]
    Colon,
    #[token("goto")]
    Goto,
    #[token("do")]
    Do,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("switch")]
    Switch,
    #[token("case")]
    Case,
    #[token("default")]
    Default,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token(",")]
    Comma,
    #[token("static")]
    Static,
    #[token("extern")]
    Extern,
    EndOfInput,
}

#[derive(Debug)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
}

pub fn lex(source: &str) -> Result<Vec<SpannedToken>, Vec<Diagnostic>> {
    let mut tokens = vec![];
    let mut errors = vec![];

    for (result, span) in Token::lexer(source).spanned() {
        match result {
            Ok(token) => tokens.push(SpannedToken { token, span }),
            Err(_) => {
                let text: String = source[span.clone()]
                    .chars()
                    .map(|c| c.escape_debug().to_string())
                    .collect();
                errors.push(Diagnostic::new(
                    Stage::Lex,
                    span,
                    format!("unexpected character '{text}'"),
                ));
            }
        }
    }

    tokens.push(SpannedToken {
        token: Token::EndOfInput,
        span: source.len()..source.len(),
    });

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

impl Token {
    pub fn is_specifier(self) -> bool {
        matches!(self, Token::Int | Token::Static | Token::Extern)
    }

    pub fn specifier(self) -> Option<Specifier> {
        match self {
            Token::Int => Some(Specifier::Int),
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
            Token::ConstantInt => "constant",
            Token::Int => "int",
            Token::Void => "void",
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
            Token::EndOfInput => "end of input",
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
