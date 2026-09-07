use std::collections::HashSet;
use std::fmt::Display;

use crate::diagnostics::{Diagnostic, Span, Stage};
use crate::lexer::{SpannedToken, Token};

pub use ast::*;
mod ast;
mod debug;
mod lowering;
mod operators;
pub use operators::*;

pub struct Parser {
    source: String,
    tokens: Vec<SpannedToken>,
    errors: Vec<Diagnostic>,
    i: usize,
}

fn expected_msg(expected: impl Display, found: Token) -> String {
    if found == Token::EndOfInput {
        format!("expected {expected} but reached end of input")
    } else {
        format!("expected {expected} but found '{found}'")
    }
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
        let mut functions = vec![];
        while self.peek().is_some_and(|&t| t != Token::EndOfInput) {
            functions.push(self.function_declaration()?);
        }

        Some(Program(functions))
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors
            .push(Diagnostic::new(Stage::Parse, span, message));
    }

    fn peek_span(&self) -> Option<Span> {
        Some(self.tokens.get(self.i)?.span.clone())
    }

    fn prev_span(&self) -> Option<Span> {
        Some(self.tokens.get(self.i.checked_sub(1)?)?.span.clone())
    }

    fn eof_span(&self) -> Span {
        self.source.len()..self.source.len()
    }

    fn peek_span_or_eof(&self) -> Span {
        self.peek_span().unwrap_or_else(|| self.eof_span())
    }

    pub fn block(&mut self) -> Option<Block> {
        self.consume(Token::OpenBrace)?;
        let statements = self.block_items()?;
        self.consume(Token::CloseBrace)?;

        Some(statements)
    }

    pub fn block_items(&mut self) -> Option<Block> {
        let mut items = vec![];

        while !self.peek()?.is_close_brace() {
            items.push(self.block_item()?);
        }

        Some(items)
    }

    pub fn block_item(&mut self) -> Option<BlockItem> {
        if self.peek()?.is_int() {
            self.declaration().map(BlockItem::Decl)
        } else {
            self.statement().map(BlockItem::Stmt)
        }
    }

    pub fn function_declaration(&mut self) -> Option<FunctionDeclaration> {
        let start = self.peek_span()?;
        self.consume_with_custom_expected_message(Token::Int, "function declaration")?;
        let name_span = self.peek_span()?;
        let name = self.ident()?;

        self.consume(Token::OpenParen)?;
        let args = self.param_list()?;
        self.consume(Token::CloseParen)?;

        let block = match self.peek()? {
            Token::OpenBrace => Some(self.block()?),
            Token::Semicolon => {
                self.next()?;
                None
            }
            _ => {
                self.error(
                    self.peek_span()?,
                    format!(
                        "Unexpected token {}. Expected function body or ;",
                        self.peek().unwrap()
                    ),
                );

                return None;
            }
        };

        let span = start.start..self.prev_span()?.end;

        Some(FunctionDeclaration {
            name,
            params: args,
            body: block,
            span,
            name_span,
        })
    }

    pub fn variable_declaration(&mut self) -> Option<VariableDeclaration> {
        let start = self.peek_span()?;
        self.consume(Token::Int)?;
        let name = self.ident()?;

        let mut init = None;

        if self.consume_if_present(Token::Assign).is_some() {
            init = Some(self.expression(0)?);
        }

        self.consume(Token::Semicolon)?;
        let span = start.start..self.prev_span()?.end;

        Some(VariableDeclaration { name, init, span })
    }

    pub fn declaration(&mut self) -> Option<Declaration> {
        let start = self.i;

        self.consume(Token::Int)?;
        self.consume(Token::Ident)?;

        match self.peek()? {
            Token::Assign | Token::Semicolon => {
                self.i = start;
                self.variable_declaration().map(Declaration::Var)
            }
            Token::OpenParen => {
                self.i = start;
                self.function_declaration().map(Declaration::Func)
            }
            _ => {
                let found = self.peek().copied().unwrap_or(Token::EndOfInput);
                let span = self.peek_span_or_eof();
                self.error(span, {
                    let found = found;
                    if found == Token::EndOfInput {
                        format!("expected '=', ';' or '(' but reached end of input")
                    } else {
                        format!("expected = or ( but found '{found}'")
                    }
                });
                None
            }
        }
    }

    pub fn param_list(&mut self) -> Option<Vec<FunctionParameter>> {
        let consume_next = |s: &mut Self, args: &mut Vec<FunctionParameter>| -> Option<()> {
            s.consume(Token::Int)?;
            let span = s.peek_span()?;
            let name = s.ident()?;
            args.push(FunctionParameter { span, name });
            Some(())
        };

        if self.peek()?.is_void() {
            self.next()?;
            Some(vec![])
        } else {
            let mut args = vec![];
            consume_next(self, &mut args)?;

            while self.peek().is_some_and(|t| !t.is_close_paren()) {
                self.consume(Token::Comma)?;
                consume_next(self, &mut args)?;
            }

            Some(args)
        }
    }

    pub fn arguement_list(&mut self) -> Option<Vec<Expression>> {
        if self.peek().is_some_and(|t| t.is_close_paren()) {
            return Some(vec![]);
        }

        let mut args = vec![];

        let expr = self.expression(0)?;
        args.push(expr);

        while self.peek().is_some_and(|t| !t.is_close_paren()) {
            self.consume(Token::Comma)?;
            let expr = self.expression(0)?;
            args.push(expr);
        }

        Some(args)
    }

    pub fn statement(&mut self) -> Option<Statement> {
        match self.peek()? {
            Token::OpenBrace => {
                let start = self.peek_span()?;
                let block = self.block()?;
                let span = start.start..self.prev_span()?.end;
                Some(Statement::new(StmtKind::Compound(block), span))
            }
            Token::Ident if self.double_peek() == Some(&Token::Colon) => {
                let name = self.ident()?.with_suffix(".goto_label").local();
                let start = self.prev_span()?;
                self.next()?;
                let stmt = Box::new(self.statement()?);
                let span = start.start..stmt.span.end;
                Some(Statement::new(StmtKind::Label(name, stmt), span))
            }
            Token::Goto => {
                let start = self.peek_span()?;
                self.next()?;
                let name = self.ident()?.with_suffix(".goto_label").local();
                self.consume(Token::Semicolon)?;
                let span = start.start..self.prev_span()?.end;
                Some(Statement::new(StmtKind::Goto(name), span))
            }
            Token::If => {
                let start = self.peek_span()?;
                self.next()?;
                self.consume(Token::OpenParen)?;
                let cond = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                let then = Box::new(self.statement()?);

                let mut end = then.span.end;
                let mut else_ = None;
                if self.consume_if_present(Token::Else).is_some() {
                    let else_stmt = self.statement()?;
                    end = else_stmt.span.end;
                    else_ = Some(Box::new(else_stmt));
                }

                Some(Statement::new(
                    StmtKind::If { cond, then, else_ },
                    start.start..end,
                ))
            }
            Token::Return => {
                let start = self.peek_span()?;
                self.next()?;
                let return_val = self.expression(0)?;
                self.consume(Token::Semicolon)?;
                let span = start.start..self.prev_span()?.end;
                Some(Statement::new(StmtKind::Return(return_val), span))
            }
            Token::Semicolon => {
                let span = self.peek_span()?;
                self.next()?;
                Some(Statement::new(StmtKind::Null, span))
            }
            Token::Break => {
                let span = self.peek_span()?;
                self.next()?;
                self.consume(Token::Semicolon)?;
                Some(Statement::new(StmtKind::Break(Identifier::dummy()), span))
            }
            Token::Continue => {
                let span = self.peek_span()?;
                self.next()?;
                self.consume(Token::Semicolon)?;
                Some(Statement::new(
                    StmtKind::Continue(Identifier::dummy()),
                    span,
                ))
            }
            Token::While => {
                let start = self.peek_span()?;
                self.next()?;
                self.consume(Token::OpenParen)?;
                let cond = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                let body = Box::new(self.statement()?);
                let span = start.start..body.span.end;
                Some(Statement::new(
                    StmtKind::While {
                        cond,
                        body,
                        label: Identifier::new("while"),
                    },
                    span,
                ))
            }
            Token::Do => {
                let start = self.peek_span()?;
                self.next()?;
                let body = Box::new(self.statement()?);
                self.consume(Token::While)?;
                self.consume(Token::OpenParen)?;
                let cond = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                self.consume(Token::Semicolon)?;
                let span = start.start..self.prev_span()?.end;

                Some(Statement::new(
                    StmtKind::DoWhile {
                        body,
                        cond,
                        label: Identifier::new("do_while"),
                    },
                    span,
                ))
            }
            Token::For => {
                let start = self.peek_span()?;
                self.next()?;
                self.consume(Token::OpenParen)?;

                let init = self.for_init()?;
                let condition = self.expression_or_nothing();
                self.consume(Token::Semicolon)?;
                let post = self.expression_or_nothing();
                self.consume(Token::CloseParen)?;
                let body = Box::new(self.statement()?);
                let span = start.start..body.span.end;

                Some(Statement::new(
                    StmtKind::For {
                        init,
                        condition,
                        post,
                        body,
                        label: Identifier::new("for"),
                    },
                    span,
                ))
            }
            Token::Switch => {
                let start = self.peek_span()?;
                self.next()?;
                self.consume(Token::OpenParen)?;
                let value = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                let body = Box::new(self.statement()?);
                let span = start.start..body.span.end;

                Some(Statement::new(
                    StmtKind::Switch(Switch {
                        value,
                        label: Identifier::new("switch"),
                        body,
                        case_set: HashSet::new(),
                        cases: Vec::new(),
                        default_case: None,
                    }),
                    span,
                ))
            }
            Token::Case => {
                let start = self.peek_span()?;
                self.next()?;

                if self.peek() != Some(&Token::ConstantInt) {
                    let found = self.peek().copied().unwrap_or(Token::EndOfInput);
                    let span = self.peek_span_or_eof();
                    self.error(span, expected_msg(Token::ConstantInt, found));
                    return None;
                }
                self.next()?;
                let value = self.constant()?;

                self.consume(Token::Colon)?;
                let header_end = self.prev_span()?.end;
                let stmt = Box::new(self.statement()?);
                let span = start.start..stmt.span.end;
                Some(Statement::new(
                    StmtKind::Case {
                        value,
                        label: Identifier::new("case"),
                        header_span: start.start..header_end,
                        stmt,
                    },
                    span,
                ))
            }
            Token::Default => {
                let start = self.peek_span()?;
                self.next()?;
                self.consume(Token::Colon)?;
                let header_end = self.prev_span()?.end;
                let stmt = Box::new(self.statement()?);
                let span = start.start..stmt.span.end;
                Some(Statement::new(
                    StmtKind::DefaultCase {
                        label: Identifier::new("default_case"),
                        header_span: start.start..header_end,
                        stmt,
                    },
                    span,
                ))
            }
            _ => {
                let expr = self.expression(0)?;
                self.consume(Token::Semicolon)?;
                let span = expr.span.clone();
                Some(Statement::new(StmtKind::Expression(expr), span))
            }
        }
    }

    /// consumes semicolon
    pub fn for_init(&mut self) -> Option<ForInit> {
        if self.peek() == Some(&Token::Semicolon) {
            self.next()?;
            return Some(ForInit::None);
        }

        if self.peek() == Some(&Token::Int) {
            let decl = self.variable_declaration()?;
            return Some(ForInit::Decl(decl));
        }

        let expr = self.expression(0)?;
        self.consume(Token::Semicolon)?;
        Some(ForInit::Expr(expr))
    }

    pub fn expression_or_nothing(&mut self) -> Option<Expression> {
        match self.peek()? {
            Token::Semicolon | Token::CloseParen => None,
            _ => self.expression(0),
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
                let span = lhs.span.start..rhs.span.end;
                lhs = Expression::new(
                    ExprKind::Conditional(Box::new(lhs), Box::new(mhs), Box::new(rhs)),
                    span,
                );
            } else if let Some(operator) = self.peek_binary_operator() {
                if operator.precedence() < min_precedence {
                    break;
                }

                self.next()?;

                if operator.is_compound_assign() {
                    let rhs = self.expression(operator.precedence())?;
                    let span = lhs.span.start..rhs.span.end;
                    lhs = Expression::new(
                        ExprKind::CompoundAssign {
                            operator,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span,
                    );
                } else if operator.is_assign() {
                    let rhs = self.expression(operator.precedence())?;
                    let span = lhs.span.start..rhs.span.end;
                    lhs = Expression::new(ExprKind::Assignment(Box::new(lhs), Box::new(rhs)), span);
                } else {
                    let rhs = self.expression(operator.precedence() + 1)?;
                    let span = lhs.span.start..rhs.span.end;
                    lhs = Expression::new(
                        ExprKind::Binary {
                            operator,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span,
                    );
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
                let s = self.source[span.clone()].to_string();

                let expr;
                if self.peek().is_some_and(|t| t.is_open_paren()) {
                    self.next()?;
                    let args = self.arguement_list()?;
                    self.consume(Token::CloseParen)?;

                    let span = span.start..self.prev_span()?.end;
                    expr = Expression::new(
                        ExprKind::FunctionCall {
                            name: Identifier::new_raw(&s),
                            args,
                        },
                        span,
                    );
                } else {
                    expr = Expression::new(ExprKind::Var(Identifier::new_raw(&s)), span);
                }

                self.postfix(expr)
            }
            Token::OpenParen => {
                let open = self.current_spanned()?.span.start;
                let expr = self.expression(0)?;
                self.consume(Token::CloseParen)?;
                let close = self.prev_span()?.end;
                let parenthesized = Expression::new(expr.kind, open..close);
                self.postfix(parenthesized)
            }
            Token::ConstantInt => {
                // it doesnt make sense to have a postfix operator on a constant, but we look for it anyway,
                // so that if this is done, we give a more useful error like "invalid lvalue", instead of "unexpected characters"
                let span = self.current_spanned()?.span.clone();
                let expr = Expression::new(ExprKind::Constant(self.constant()?), span);
                self.postfix(expr)
            }
            Token::Hyphen => {
                let start = self.current_spanned()?.span.start;
                self.unary(UnaryOperator::Negate, start)
            }
            Token::Tilde => {
                let start = self.current_spanned()?.span.start;
                self.unary(UnaryOperator::BitwiseNot, start)
            }
            Token::Not => {
                let start = self.current_spanned()?.span.start;
                self.unary(UnaryOperator::Not, start)
            }
            Token::Increment => {
                let start = self.current_spanned()?.span.start;
                self.prefix_inc_dec(IncDec::Increment, start)
            }
            Token::Decrement => {
                let start = self.current_spanned()?.span.start;
                self.prefix_inc_dec(IncDec::Decrement, start)
            }
            _ => {
                let found = self.current().copied().unwrap_or(Token::EndOfInput);
                let span = self
                    .current_spanned()
                    .map(|t| t.span.clone())
                    .unwrap_or_else(|| self.eof_span());
                let message = if found == Token::EndOfInput {
                    "unexpected end of input".to_string()
                } else {
                    format!("unexpected token '{found}', expected expression")
                };
                self.error(span, message);

                None
            }
        }
    }

    fn unary(&mut self, operator: UnaryOperator, start: usize) -> Option<Expression> {
        let expr = Box::new(self.factor()?);
        let span = start..expr.span.end;
        Some(Expression::new(ExprKind::Unary { operator, expr }, span))
    }

    fn prefix_inc_dec(&mut self, op: IncDec, start: usize) -> Option<Expression> {
        let expr = Box::new(self.factor()?);
        let span = start..expr.span.end;
        Some(Expression::new(ExprKind::Prefix(op, expr), span))
    }

    fn postfix(&mut self, mut expr: Expression) -> Option<Expression> {
        loop {
            let op = match self.peek() {
                Some(Token::Increment) => IncDec::Increment,
                Some(Token::Decrement) => IncDec::Decrement,
                _ => break,
            };

            let end = self.peek_span()?.end;
            self.next()?;
            let span = expr.span.start..end;
            expr = Expression::new(ExprKind::Postfix(op, Box::new(expr)), span);
        }

        Some(expr)
    }

    /// parses the constant at the current position without consuming anything
    pub fn constant(&mut self) -> Option<Constant> {
        let span = self.current_spanned()?.span.clone();

        match self.source[span.clone()].parse::<i32>() {
            Ok(value) => Some(Constant::Int(value)),
            Err(_) => {
                self.error(span, "integer constant is too large to fit in an 'int'");
                None
            }
        }
    }

    pub fn ident(&mut self) -> Option<Identifier> {
        self.consume(Token::Ident)?;

        match self.current_spanned()? {
            SpannedToken {
                token: Token::Ident,
                span,
            } => {
                let value = &self.source[span.clone()];
                Some(Identifier::new_raw(value))
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
            let token = &self.tokens[self.i].token;
            self.i += 1;
            Some(token)
        } else {
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
        if self.peek() == Some(&token) {
            self.next();
            Some(())
        } else {
            None
        }
    }

    pub fn consume(&mut self, expected: Token) -> Option<()> {
        match self.peek() {
            Some(&found) if found == expected => {
                self.i += 1;
                Some(())
            }
            Some(&found) => {
                let span = self.peek_span()?;
                self.error(span, expected_msg(expected, found));
                None
            }
            None => {
                let span = self.eof_span();
                self.error(span, expected_msg(expected, Token::EndOfInput));
                None
            }
        }
    }

    pub fn consume_with_custom_expected_message(
        &mut self,
        expected: Token,
        msg: impl Display,
    ) -> Option<()> {
        match self.peek() {
            Some(&found) if found == expected => {
                self.i += 1;
                Some(())
            }
            Some(&found) => {
                let span = self.peek_span()?;
                self.error(span, expected_msg(msg, found));
                None
            }
            None => {
                let span = self.eof_span();
                self.error(span, expected_msg(msg, Token::EndOfInput));
                None
            }
        }
    }

    pub fn peek_binary_operator(&self) -> Option<BinaryOperator> {
        let token = self.peek()?;
        BinaryOperator::from_token(*token)
    }
}

pub fn parse(source: String, tokens: Vec<SpannedToken>) -> Result<Program, Vec<Diagnostic>> {
    let mut parser = Parser::new(source, tokens);
    let program = parser.parse();

    match program {
        Some(program) if parser.errors.is_empty() => Ok(program),
        _ => {
            if parser.errors.is_empty() {
                let span = parser.eof_span();
                parser.errors.push(Diagnostic::new(
                    Stage::Parse,
                    span,
                    "unexpected end of input",
                ));
            }
            Err(parser.errors)
        }
    }
}
