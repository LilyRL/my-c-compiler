use std::collections::HashSet;

use super::operators::{BinaryOperator, IncDec, UnaryOperator};
use crate::diagnostics::Span;

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

pub type Block = Vec<BlockItem>;

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub block: Block,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Identifier(pub String);

impl Identifier {
    fn rand() -> String {
        (0..4).map(|_| rand::random_range('a'..'z')).collect()
    }

    pub fn new(name: &str) -> Self {
        #[cfg(target_os = "linux")]
        return Self(format!(".L_{name}__{}", Self::rand()));
        #[cfg(target_os = "macos")]
        return Self(format!("L_{name}__{}", Self::rand()));
    }

    pub fn local(&self) -> Self {
        #[cfg(target_os = "linux")]
        return Self(format!(".L_{}", self.0));
        #[cfg(target_os = "macos")]
        return Self(format!("L_{}", self.0));
    }

    pub fn with_suffix(&self, suffix: &str) -> Self {
        return Self(format!("{}{suffix}", self.0));
    }

    pub fn _start(&self) -> Self {
        self.with_suffix("_start")
    }

    pub fn _break(&self) -> Self {
        self.with_suffix("_break")
    }

    pub fn _continue(&self) -> Self {
        self.with_suffix("_continue")
    }

    pub fn dummy() -> Self {
        return Self("DUMMY_IDENTIFIER__SHOULD_NOT_APPEAR_IN_OUTPUT".to_string());
    }
}

#[derive(Debug)]
pub enum BlockItem {
    Stmt(Statement),
    Decl(Declaration),
}

#[derive(Debug)]
pub struct Statement {
    pub kind: StmtKind,
    pub span: Span,
}

impl Statement {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug)]
pub enum StmtKind {
    Return(Expression),
    Expression(Expression),
    If {
        cond: Expression,
        then: Box<Statement>,
        else_: Option<Box<Statement>>,
    },
    Null,
    Goto(Identifier),
    Label(Identifier, Box<Statement>),
    Compound(Block),
    Break(Identifier),
    Continue(Identifier),
    While {
        cond: Expression,
        body: Box<Statement>,
        label: Identifier,
    },
    DoWhile {
        body: Box<Statement>,
        cond: Expression,
        label: Identifier,
    },
    For {
        init: ForInit,
        condition: Option<Expression>,
        post: Option<Expression>,
        body: Box<Statement>,
        label: Identifier,
    },
    Switch(Switch),
    Case {
        value: Constant,
        label: Identifier,
        /// span excluding stmt
        header_span: Span,
        stmt: Box<Statement>,
    },
    DefaultCase {
        label: Identifier,
        header_span: Span,
        stmt: Box<Statement>,
    },
}

#[derive(Debug)]
pub struct Switch {
    pub value: Expression,
    pub body: Box<Statement>,
    pub label: Identifier,

    pub case_set: HashSet<Constant>,
    pub cases: Vec<SwitchCase>,
    pub default_case: Option<Identifier>,
}

pub type SwitchCase = (Identifier, Constant);

#[derive(Debug)]
pub enum ForInit {
    Decl(Declaration),
    Expr(Expression),
    None,
}

#[derive(Debug)]
pub struct Declaration {
    pub name: Identifier,
    pub init: Option<Expression>,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Var(Identifier),
    Constant(Constant),
    Unary {
        operator: UnaryOperator,
        expr: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    CompoundAssign {
        operator: BinaryOperator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    Assignment(Box<Expression>, Box<Expression>),
    Prefix(IncDec, Box<Expression>),
    Postfix(IncDec, Box<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
}

#[derive(Debug)]
pub struct Expression {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expression {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn is_var(&self) -> bool {
        matches!(self.kind, ExprKind::Var(_))
    }

    pub fn as_var(&self) -> Option<&Identifier> {
        match &self.kind {
            ExprKind::Var(i) => Some(i),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord, Clone)]
pub enum Constant {
    Int(i32),
}

impl Constant {
    pub fn i32(self) -> i32 {
        match self {
            Self::Int(i) => i,
        }
    }
}
