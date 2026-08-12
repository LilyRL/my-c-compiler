use std::ops::Range;

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
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
    #[token("--")]
    Decrement,
    #[token("+")]
    Plus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
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
