use std::ops::Range;

use crate::lexer::{SpannedToken, Token};

pub use data::*;
mod data;

pub struct Parser {
    source: String,
    tokens: Vec<SpannedToken>,
    errors: Vec<Error>,
    i: usize,
}

#[derive(Debug)]
pub enum Error {
    ExpectedToken {
        expected: Token,
        found: Token,
        span: Range<usize>,
    },
    UnexpectedToken {
        found: Token,
        span: Range<usize>,
    },
}

impl Parser {
    pub fn new(source: String, tokens: Vec<SpannedToken>) -> Self {
        Self {
            source,
            tokens,
            i: 0,
            errors: vec![],
        }
    }

    pub fn parse(&mut self) -> Option<Program> {
        let function = self.function_definition()?;
        self.consume(Token::EndOfInput)?;

        Some(Program(function))
    }

    pub fn function_definition(&mut self) -> Option<FunctionDefinition> {
        self.consume(Token::Int)?;
        let name = self.ident()?;
        self.consume(Token::OpenParen)?;
        self.consume_if_present(Token::Void);
        self.consume(Token::CloseParen)?;
        let block = self.block()?;

        Some(FunctionDefinition { name, block })
    }

    pub fn block(&mut self) -> Option<Statement> {
        self.consume(Token::OpenBrace)?;
        let stmt = self.statement()?;
        self.consume(Token::CloseBrace)?;

        Some(stmt)
    }

    pub fn statement(&mut self) -> Option<Statement> {
        self.consume(Token::Return)?;
        let return_val = self.expression()?;
        self.consume(Token::Semicolon)?;

        Some(Statement::Return(return_val))
    }

    pub fn expression(&mut self) -> Option<Expression> {
        match self.next()? {
            Token::OpenParen => {
                let expr = self.expression()?;
                self.consume(Token::CloseParen)?;
                Some(expr)
            }
            Token::ConstantInt => Some(Expression::Constant(self.constant()?)),
            Token::Hyphen => {
                let expr = self.expression()?;
                Some(Expression::Unary {
                    operator: UnaryOperator::Negate,
                    expr: Box::new(expr),
                })
            }
            Token::Tilde => {
                let expr = self.expression()?;
                Some(Expression::Unary {
                    operator: UnaryOperator::Complement,
                    expr: Box::new(expr),
                })
            }
            _ => {
                let current = self.current_spanned();
                self.errors.push(Error::UnexpectedToken {
                    found: self.current()?.clone(),
                    span: current?.span.clone(),
                });

                None
            }
        }
    }

    pub fn constant(&mut self) -> Option<Constant> {
        Some(Constant::Int(self.int()?))
    }

    pub fn int(&mut self) -> Option<i32> {
        match self.current_spanned()? {
            SpannedToken {
                token: Token::ConstantInt,
                span,
            } => {
                let value = self.source[span.clone()].parse::<i32>().ok()?;
                Some(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn ident(&mut self) -> Option<Identifier> {
        self.consume(Token::Ident)?;

        match self.current_spanned()? {
            SpannedToken {
                token: Token::Ident,
                span,
            } => {
                let value = self.source[span.clone()].to_string();
                Some(Identifier(value))
            }
            _ => unreachable!(),
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.i >= self.tokens.len()
    }

    pub fn next(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            let token = &self.tokens[self.i];
            self.i += 1;
            Some(&token.token)
        } else {
            self.errors.push(Error::UnexpectedToken {
                found: Token::EndOfInput,
                span: self.source.len()..self.source.len(),
            });
            None
        }
    }

    pub fn current(&self) -> Option<&Token> {
        if self.i > 0 {
            Some(&self.tokens[self.i - 1].token)
        } else {
            None
        }
    }

    pub fn current_spanned(&self) -> Option<&SpannedToken> {
        if self.i > 0 {
            Some(&self.tokens[self.i - 1])
        } else {
            None
        }
    }

    pub fn peek(&self) -> Option<&Token> {
        if !self.is_at_end() {
            Some(&self.tokens[self.i].token)
        } else {
            None
        }
    }

    pub fn double_peek(&self) -> Option<&Token> {
        if self.i + 1 < self.tokens.len() {
            Some(&self.tokens[self.i + 1].token)
        } else {
            None
        }
    }

    pub fn consume_if_present(&mut self, token: Token) {
        if let Some(current) = self.peek() {
            if *current == token {
                self.next();
            }
        }
    }

    pub fn consume(&mut self, expected: Token) -> Option<()> {
        if let Some(&found) = self.next() {
            if found == expected {
                Some(())
            } else {
                let span = self.tokens[self.i - 1].span.clone();
                self.errors.push(Error::ExpectedToken {
                    expected,
                    found,
                    span,
                });
                None
            }
        } else {
            let span = if self.i > 0 {
                self.tokens[self.i - 1].span.clone()
            } else {
                0..0
            };
            self.errors.push(Error::ExpectedToken {
                expected,
                found: Token::EndOfInput,
                span,
            });
            None
        }
    }
}

pub fn parse(source: String, tokens: Vec<SpannedToken>) -> Option<Program> {
    let mut parser = Parser::new(source, tokens);
    let program = parser.parse();

    if !parser.errors.is_empty() {
        for error in parser.errors {
            // TODO: replace with some ariadne pretty printing
            println!("{:?}", error);
        }
        std::process::exit(1);
    }

    program
}
