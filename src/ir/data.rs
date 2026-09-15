use crate::parser::{Constant, Identifier};

#[derive(Debug)]
pub struct Program(pub Vec<TopLevel>);

#[derive(Debug)]
pub enum TopLevel {
    F(FunctionDefinition),
    V(StaticVariable),
}

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub params: Vec<Identifier>,
    pub body: Vec<Instruction>,
    pub global: bool,
}

#[derive(Debug)]
pub struct StaticVariable {
    pub name: Identifier,
    pub global: bool,
    pub init: Constant,
}

#[derive(Debug)]
pub enum Instruction {
    Return(Value),
    Unary {
        operator: UnaryOperator,
        src: Value,
        dst: Value,
    },
    Binary {
        operator: BinaryOperator,
        lhs: Value,
        rhs: Value,
        dst: Value,
    },
    Copy {
        src: Value,
        dst: Value,
    },
    Jump(Identifier),
    JumpIfZero {
        condition: Value,
        target: Identifier,
    },
    JumpNotZero {
        condition: Value,
        target: Identifier,
    },
    Label(Identifier),
    Comment(&'static str),
    FunctionCall {
        name: Identifier,
        args: Vec<Value>,
        dst: Value,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    BitwiseNot,
    Negate,
    Not,
}

#[derive(Clone, Debug)]
pub enum Value {
    Constant(i32),
    Var(Identifier),
}
