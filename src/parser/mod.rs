use std::ops::Range;

use crate::lexer::{SpannedToken, Token};

pub use ast::*;
use thiserror::Error;

mod ast;
mod debug;
mod lowering;

pub struct Parser {
    source: String,
    tokens: Vec<SpannedToken>,
    errors: Vec<Error>,
    i: usize,
}

pub struct Error {
    ty: ErrorType,
    span: Range<usize>,
}

#[derive(Debug, Error)]
pub enum ErrorType {
    #[error("Unexpected token. Expected {expected} found {found}")]
    ExpectedToken { expected: Token, found: Token },
    #[error("Unexpected token. Found {found}")]
    UnexpectedToken { found: Token },
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

    pub fn print_errors(&self, file_name: &str) {
        use ariadne::{Label, Report, ReportKind, Source};

        for error in &self.errors {
            Report::build(ReportKind::Error, (file_name, error.span.clone()))
                .with_label(
                    Label::new((file_name, error.span.clone()))
                        .with_message(format!("{}", error.ty)),
                )
                .finish()
                .print((file_name, Source::from(&self.source)))
                .unwrap();
        }
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

    pub fn block(&mut self) -> Option<Block> {
        self.consume(Token::OpenBrace)?;
        let statements = self.block_items()?;
        self.consume(Token::CloseBrace)?;

        Some(statements)
    }

    pub fn block_items(&mut self) -> Option<Vec<BlockItem>> {
        let mut items = vec![];

        while !self.peek()?.is_close_brace() {
            items.push(self.block_item()?);
        }

        Some(items)
    }

    pub fn block_item(&mut self) -> Option<BlockItem> {
        if self.peek()?.is_int() {
            self.declaration().map(|d| BlockItem::Decl(d))
        } else {
            self.statement().map(|s| BlockItem::Stmt(s))
        }
    }

    pub fn declaration(&mut self) -> Option<Declaration> {
        self.consume(Token::Int)?;
        let name = self.ident()?;
        let mut init = None;

        if self.consume_if_present(Token::Assign).is_some() {
            init = Some(self.expression(0)?);
        }

        self.consume(Token::Semicolon)?;

        Some(Declaration { name, init })
    }

    pub fn statement(&mut self) -> Option<Statement> {
        match self.peek()? {
            Token::OpenBrace => {
                let block = self.block()?;
                Some(Statement::Compound(block))
            }
            Token::Ident if self.double_peek() == Some(&Token::Colon) => {
                let name = self.ident()?.with_suffix(".goto_label");
                self.next()?;
                Some(Statement::Label(name))
            }
            Token::Goto => {
                self.next()?;
                let name = self.ident()?.with_suffix(".goto_label");
                self.consume(Token::Semicolon)?;
                Some(Statement::Goto(name))
            }
            Token::If => {
                self.next()?;
                self.consume(Token::OpenParen)?;
                let cond = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                let then = Box::new(self.statement()?);

                let mut else_ = None;
                if self.consume_if_present(Token::Else).is_some() {
                    else_ = Some(Box::new(self.statement()?));
                }

                Some(Statement::If { cond, then, else_ })
            }
            Token::Return => {
                self.next()?;
                let return_val = self.expression(0)?;
                self.consume(Token::Semicolon)?;
                Some(Statement::Return(return_val))
            }
            Token::Semicolon => {
                self.next()?;
                Some(Statement::Null)
            }
            _ => {
                let expr = self.expression(0)?;
                self.consume(Token::Semicolon)?;
                Some(Statement::Expression(expr))
            }
        }
    }

    pub fn expression(&mut self, min_precedence: u32) -> Option<Expression> {
        let mut lhs = self.factor()?;

        loop {
            if self.peek() == Some(&Token::QuestionMark) && CONDITIONAL_PRECEDENCE > min_precedence
            {
                self.next()?;
                let mhs = self.expression(0)?;
                self.consume(Token::Colon)?;
                let rhs = self.expression(CONDITIONAL_PRECEDENCE)?;
                lhs = Expression::Conditional(Box::new(lhs), Box::new(mhs), Box::new(rhs));
            } else if let Some(operator) = self.peek_binary_operator() {
                if operator.precedence() < min_precedence {
                    break;
                }

                self.next()?;

                if operator.is_compound_assign() {
                    let rhs = self.expression(operator.precedence())?;
                    lhs = Expression::CompoundAssign {
                        operator,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                } else if operator.is_assign() {
                    let rhs = self.expression(operator.precedence())?;
                    lhs = Expression::Assignment(Box::new(lhs), Box::new(rhs));
                } else {
                    let rhs = self.expression(operator.precedence() + 1)?;

                    lhs = Expression::Binary {
                        operator,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
            } else {
                break;
            }
        }

        Some(lhs)
    }

    pub fn factor(&mut self) -> Option<Expression> {
        match self.next()? {
            Token::Ident => {
                let span = self.current_spanned()?.span.clone();
                let s = self.source[span].to_string();
                self.postfix(Expression::Var(Identifier(s)))
            }
            Token::OpenParen => {
                let expr = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                self.postfix(expr)
            }
            Token::ConstantInt => {
                // it doesnt make sense to have a postfix operator on a constant, but we look for it anyway,
                // so that if this is done, we give a more useful error like "invalid lvalue", instead of "unexpected characters"
                let expr = Expression::Constant(self.constant()?);
                self.postfix(expr)
            }
            Token::Hyphen => {
                let expr = self.factor()?;
                Some(Expression::Unary {
                    operator: UnaryOperator::Negate,
                    expr: Box::new(expr),
                })
            }
            Token::Decrement => {
                let expr = self.factor()?;
                Some(Expression::Prefix(IncDec::Decrement, Box::new(expr)))
            }
            Token::Increment => {
                let expr = self.factor()?;
                Some(Expression::Prefix(IncDec::Increment, Box::new(expr)))
            }
            Token::Tilde => {
                let expr = self.factor()?;
                Some(Expression::Unary {
                    operator: UnaryOperator::BitwiseNot,
                    expr: Box::new(expr),
                })
            }
            Token::Not => {
                let expr = self.factor()?;
                Some(Expression::Unary {
                    operator: UnaryOperator::Not,
                    expr: Box::new(expr),
                })
            }
            _ => {
                let current = self.current_spanned();
                self.errors.push(Error {
                    span: current?.span.clone(),
                    ty: ErrorType::UnexpectedToken {
                        found: self.current()?.clone(),
                    },
                });

                None
            }
        }
    }

    fn postfix(&mut self, mut expr: Expression) -> Option<Expression> {
        loop {
            match self.peek() {
                Some(Token::Increment) => {
                    self.next();
                    expr = Expression::Postfix(IncDec::Increment, Box::new(expr));
                }
                Some(Token::Decrement) => {
                    self.next();
                    expr = Expression::Postfix(IncDec::Decrement, Box::new(expr));
                }
                _ => break,
            }
        }
        Some(expr)
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

    pub fn is_nearly_at_end(&self) -> bool {
        self.i + 1 >= self.tokens.len()
    }

    pub fn next(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            let token = &self.tokens[self.i];
            self.i += 1;
            Some(&token.token)
        } else {
            self.errors.push(Error {
                ty: ErrorType::UnexpectedToken {
                    found: Token::EndOfInput,
                },
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
        if !self.is_nearly_at_end() {
            Some(&self.tokens[self.i + 1].token)
        } else {
            None
        }
    }

    pub fn consume_if_present(&mut self, token: Token) -> Option<()> {
        if let Some(current) = self.peek() {
            if *current == token {
                self.next();
                return Some(());
            } else {
            }
        }

        None
    }

    pub fn consume(&mut self, expected: Token) -> Option<()> {
        if let Some(&found) = self.next() {
            if found == expected {
                Some(())
            } else {
                let span = self.tokens[self.i - 1].span.clone();
                self.errors.push(Error {
                    ty: ErrorType::ExpectedToken { expected, found },
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
            self.errors.push(Error {
                ty: ErrorType::ExpectedToken {
                    expected,
                    found: Token::EndOfInput,
                },
                span,
            });
            None
        }
    }

    pub fn peek_binary_operator(&self) -> Option<BinaryOperator> {
        let token = self.peek()?;
        BinaryOperator::from_token(*token)
    }
}

pub fn parse(source: String, tokens: Vec<SpannedToken>, file_name: &str) -> Option<Program> {
    let mut parser = Parser::new(source, tokens);
    let program = parser.parse();

    if !parser.errors.is_empty() {
        parser.print_errors(file_name);
        std::process::exit(1);
    }

    program
}
