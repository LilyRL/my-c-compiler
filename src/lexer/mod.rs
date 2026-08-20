use std::{fmt::Display, ops::Range};

use logos::Logos;
use strum::EnumIs;

#[derive(Logos, Debug, PartialEq, Copy, Clone, EnumIs)]
#[logos(skip r"[ \t\n]+")]
#[logos(error = String)]
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
    EndOfInput,
}

#[derive(Debug)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
}

pub fn lex(source: &str) -> Option<Vec<SpannedToken>> {
    let lexer = Token::lexer(source);

    let mut tokens = vec![];
    for (token, span) in lexer.spanned() {
        match token {
            Ok(token) => tokens.push(SpannedToken { token, span }),
            Err(e) => {
                println!("lexer error at {:?}: {}", span, e);
                return None;
            }
        }
    }

    tokens.push(SpannedToken {
        token: Token::EndOfInput,
        span: Range {
            start: source.len(),
            end: source.len(),
        },
    });

    Some(tokens)
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
        };
        write!(f, "{}", s)
    }
}
