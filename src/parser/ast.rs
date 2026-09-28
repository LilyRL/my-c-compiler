use std::{
    collections::HashMap,
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

use strum::{EnumDiscriminants, EnumIs, IntoDiscriminant};

use super::operators::{BinaryOperator, IncDec, UnaryOperator};
use crate::{
    analysis::{StaticInit, Type},
    diagnostics::Span,
};

#[derive(Debug)]
pub struct Program(pub Vec<Declaration>);

impl Program {
    pub fn functions(&self) -> impl Iterator<Item = &FunctionDeclaration> {
        self.0.iter().filter_map(|decl| match decl {
            Declaration::Func(func) => Some(func),
            _ => None,
        })
    }

    pub fn functions_mut(&mut self) -> impl Iterator<Item = &mut FunctionDeclaration> {
        self.0.iter_mut().filter_map(|decl| match decl {
            Declaration::Func(func) => Some(func),
            _ => None,
        })
    }
}

pub type Block = Vec<BlockItem>;

#[derive(Debug)]
pub struct FunctionDeclaration {
    pub name: Identifier,
    pub params: Vec<FunctionParameter>,
    pub body: Option<Block>,
    pub span: Span,
    pub name_span: Span,
    pub storage_class: StorageClass,
    pub return_type: Type,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Specifier {
    Int,
    Long,
    Unsigned,
    Signed,
    Static,
    Extern,
}

impl Specifier {
    pub fn storage_class(&self) -> StorageClass {
        match self {
            Specifier::Static => StorageClass::Static,
            Specifier::Extern => StorageClass::Extern,
            _ => StorageClass::None,
        }
    }
}

#[derive(Debug)]
pub struct FunctionParameter {
    pub name: Identifier,
    pub span: Span,
    pub ty: Type,
}

#[derive(Debug)]
pub struct VariableDeclaration {
    pub name: Identifier,
    pub init: Option<Expression>,
    pub span: Span,
    pub storage_class: StorageClass,
    pub ty: Type,
}

#[derive(Debug)]
pub enum Declaration {
    Func(FunctionDeclaration),
    Var(VariableDeclaration),
}

#[derive(Debug, EnumIs)]
pub enum StorageClass {
    Static,
    Extern,
    None,
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

    pub case_set: HashMap<Constant, Span>,
    pub cases: Vec<SwitchCase>,
    pub default_case: Option<Identifier>,
}

pub type SwitchCase = (Identifier, Constant);

#[derive(Debug)]
pub enum ForInit {
    Decl(VariableDeclaration),
    Expr(Expression),
    None,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Var(Identifier),
    Constant(Constant),
    Cast {
        target_type: Type,
        expr: Box<Expression>,
    },
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
    FunctionCall {
        name: Identifier,
        args: Vec<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub ty: Type,
    pub kind: ExprKind,
    pub span: Span,
}

impl Expression {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self {
            kind,
            span,
            ty: Type::Int,
        }
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

#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord, Clone, Copy, EnumDiscriminants)]
#[strum_discriminants(name(ConstantType))]
pub enum Constant {
    Int(i32),
    UInt(u32),
    Long(i64),
    ULong(u64),
}

impl Constant {
    pub fn from_magnitude(magnitude: u128, ty: ConstantType) -> Self {
        match ty {
            ConstantType::Int => Constant::Int(magnitude as i32),
            ConstantType::UInt => Constant::UInt(magnitude as u32),
            ConstantType::Long => Constant::Long(magnitude as i64),
            ConstantType::ULong => Constant::ULong(magnitude as u64),
        }
    }

    pub fn i64(&self) -> i64 {
        match self {
            Constant::Int(i) => *i as i64,
            Constant::Long(l) => *l,
            Constant::UInt(u) => *u as i64,
            Constant::ULong(ul) => *ul as i64,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Constant::Int(i) => *i == 0,
            Constant::Long(l) => *l == 0,
            Constant::UInt(u) => *u == 0,
            Constant::ULong(ul) => *ul == 0,
        }
    }

    pub fn from_int(i: i32, ty: ConstantType) -> Self {
        match ty {
            ConstantType::Int => Constant::Int(i),
            ConstantType::Long => Constant::Long(i as i64),
            ConstantType::UInt => Constant::UInt(i as u32),
            ConstantType::ULong => Constant::ULong(i as u64),
        }
    }

    pub fn to_common_pair(self, other: Self) -> (Self, Self) {
        if self.discriminant() == other.discriminant() {
            (self, other)
        } else {
            (self.cast_long(), other.cast_long())
        }
    }

    pub fn cast_long(self) -> Self {
        self.cast(ConstantType::Long)
    }

    fn bits(self) -> u128 {
        match self {
            Constant::Int(i) => i as i128 as u128,
            Constant::UInt(u) => u as u128,
            Constant::Long(l) => l as i128 as u128,
            Constant::ULong(ul) => ul as u128,
        }
    }

    pub fn cast(self, const_ty: ConstantType) -> Constant {
        let bits = self.bits();

        match const_ty {
            ConstantType::Int => Constant::Int(bits as i32),
            ConstantType::UInt => Constant::UInt(bits as u32),
            ConstantType::Long => Constant::Long(bits as i64),
            ConstantType::ULong => Constant::ULong(bits as u64),
        }
    }

    pub fn to_static_init(self) -> StaticInit {
        match self {
            Constant::Int(i) => StaticInit::Int(i),
            Constant::UInt(u) => StaticInit::UInt(u),
            Constant::Long(l) => StaticInit::Long(l),
            Constant::ULong(ul) => StaticInit::ULong(ul),
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Constant::Int(_) => Type::Int,
            Constant::Long(_) => Type::Long,
            Constant::UInt(_) => Type::UInt,
            Constant::ULong(_) => Type::ULong,
        }
    }
}

impl Expression {
    pub fn process_inner_expressions<S, F: Fn(&Self, &mut S)>(&self, state: &mut S, f: &F) {
        let mut stack: Vec<&Expression> = vec![self];
        while let Some(expr) = stack.pop() {
            f(expr, state);
            match &expr.kind {
                ExprKind::Cast { expr, .. }
                | ExprKind::Unary { expr, .. }
                | ExprKind::Prefix(_, expr)
                | ExprKind::Postfix(_, expr) => stack.push(expr),
                ExprKind::Binary { lhs, rhs, .. }
                | ExprKind::CompoundAssign { lhs, rhs, .. }
                | ExprKind::Assignment(lhs, rhs) => {
                    stack.push(rhs);
                    stack.push(lhs);
                }
                ExprKind::Conditional(cond, if_true, if_false) => {
                    stack.push(if_false);
                    stack.push(if_true);
                    stack.push(cond);
                }
                ExprKind::FunctionCall { args, .. } => {
                    for arg in args.iter().rev() {
                        stack.push(arg);
                    }
                }
                ExprKind::Var(_) | ExprKind::Constant(_) => {}
            }
        }
    }

    pub fn process_inner_expressions_mut<S, F: Fn(&mut Self, &mut S)>(
        &mut self,
        state: &mut S,
        f: &F,
    ) {
        let mut stack: Vec<&mut Expression> = vec![self];
        while let Some(expr) = stack.pop() {
            f(expr, state);
            match &mut expr.kind {
                ExprKind::Cast { expr, .. }
                | ExprKind::Unary { expr, .. }
                | ExprKind::Prefix(_, expr)
                | ExprKind::Postfix(_, expr) => stack.push(expr),
                ExprKind::Binary { lhs, rhs, .. }
                | ExprKind::CompoundAssign { lhs, rhs, .. }
                | ExprKind::Assignment(lhs, rhs) => {
                    stack.push(rhs);
                    stack.push(lhs);
                }
                ExprKind::Conditional(cond, if_true, if_false) => {
                    stack.push(if_false);
                    stack.push(if_true);
                    stack.push(cond);
                }
                ExprKind::FunctionCall { args, .. } => {
                    for arg in args.iter_mut().rev() {
                        stack.push(arg);
                    }
                }
                ExprKind::Var(_) | ExprKind::Constant(_) => {}
            }
        }
    }
}

impl Statement {
    pub fn process_inner_statements_mut<S, F: Fn(&mut Statement, &mut S)>(
        &mut self,
        state: &mut S,
        f: &F,
    ) {
        let mut stack: Vec<&mut Statement> = vec![self];
        while let Some(stmt) = stack.pop() {
            f(stmt, state);
            match &mut stmt.kind {
                StmtKind::For { body, .. }
                | StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. } => {
                    stack.push(body);
                }
                StmtKind::If { then, else_, .. } => {
                    stack.push(then);
                    if let Some(else_) = else_ {
                        stack.push(else_);
                    }
                }
                StmtKind::Switch(switch) => {
                    stack.push(&mut switch.body);
                }
                StmtKind::Case { stmt, .. }
                | StmtKind::DefaultCase { stmt, .. }
                | StmtKind::Label(_, stmt) => {
                    stack.push(stmt);
                }
                StmtKind::Compound(block) => {
                    for item in block.iter_mut() {
                        if let BlockItem::Stmt(stmt) = item {
                            stack.push(stmt);
                        }
                    }
                }
                StmtKind::Return(_)
                | StmtKind::Expression(_)
                | StmtKind::Null
                | StmtKind::Goto(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_) => {}
            }
        }
    }

    pub fn process_inner_expressions_mut<S, F: Fn(&mut Expression, &mut S)>(
        &mut self,
        state: &mut S,
        f: &F,
    ) {
        self.process_inner_statements_mut(state, &|stmt, state| {
            let mut stack: Vec<&mut Expression> = vec![];

            match &mut stmt.kind {
                StmtKind::Return(e)
                | StmtKind::Expression(e)
                | StmtKind::If { cond: e, .. }
                | StmtKind::While { cond: e, .. }
                | StmtKind::DoWhile { cond: e, .. } => {
                    e.process_inner_expressions_mut(state, f);
                }
                StmtKind::For {
                    init,
                    condition,
                    post,
                    ..
                } => {
                    if let ForInit::Expr(e) = init {
                        e.process_inner_expressions_mut(state, f);
                    }
                    if let Some(e) = condition {
                        e.process_inner_expressions_mut(state, f);
                    }
                    if let Some(e) = post {
                        e.process_inner_expressions_mut(state, f);
                    }
                }
                StmtKind::Switch(switch) => {
                    stack.push(&mut switch.value);
                }
                StmtKind::Null
                | StmtKind::Goto(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_)
                | StmtKind::Label(_, _)
                | StmtKind::Compound(_)
                | StmtKind::Case { .. }
                | StmtKind::DefaultCase { .. } => {}
            }
        });
    }

    pub fn process_inner_statements<S, F: Fn(&Statement, &mut S)>(&self, state: &mut S, f: &F) {
        let mut stack: Vec<&Statement> = vec![self];
        while let Some(stmt) = stack.pop() {
            f(stmt, state);
            match &stmt.kind {
                StmtKind::For { body, .. }
                | StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. } => {
                    stack.push(body);
                }
                StmtKind::If { then, else_, .. } => {
                    stack.push(then);
                    if let Some(else_) = else_ {
                        stack.push(else_);
                    }
                }
                StmtKind::Switch(switch) => {
                    stack.push(&switch.body);
                }
                StmtKind::Case { stmt, .. }
                | StmtKind::DefaultCase { stmt, .. }
                | StmtKind::Label(_, stmt) => {
                    stack.push(stmt);
                }
                StmtKind::Compound(block) => {
                    for item in block.iter().rev() {
                        if let BlockItem::Stmt(stmt) = item {
                            stack.push(stmt);
                        }
                    }
                }
                StmtKind::Return(_)
                | StmtKind::Expression(_)
                | StmtKind::Null
                | StmtKind::Goto(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_) => {}
            }
        }
    }

    pub fn process_inner_declarations<S, F: Fn(&Declaration, &mut S)>(&self, state: &mut S, f: &F) {
        let mut stack: Vec<&Statement> = vec![self];
        while let Some(stmt) = stack.pop() {
            match &stmt.kind {
                StmtKind::Compound(block) => {
                    for item in block.iter().rev() {
                        if let BlockItem::Decl(decl) = item {
                            f(decl, state);
                        } else if let BlockItem::Stmt(stmt) = item {
                            stack.push(stmt);
                        }
                    }
                }
                StmtKind::If { then, else_, .. } => {
                    stack.push(then);
                    if let Some(else_) = else_ {
                        stack.push(else_);
                    }
                }
                StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. }
                | StmtKind::For { body, .. } => {
                    stack.push(body);
                }
                StmtKind::Switch(switch) => {
                    stack.push(&switch.body);
                }
                StmtKind::Case { stmt, .. }
                | StmtKind::DefaultCase { stmt, .. }
                | StmtKind::Label(_, stmt) => {
                    stack.push(stmt);
                }
                StmtKind::Return(_)
                | StmtKind::Expression(_)
                | StmtKind::Null
                | StmtKind::Goto(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_) => {}
            }
        }
    }
}
