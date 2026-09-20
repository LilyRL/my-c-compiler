use strum::IntoDiscriminant;

use crate::{
    analysis::{StaticInit, Type, get_symbols},
    codegen::AssemblyType,
    parser::{Constant, ConstantType, FunctionParameter, Identifier},
};

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
    pub params: Vec<FunctionParameter>,
    pub body: Vec<Instruction>,
    pub global: bool,
}

#[derive(Debug)]
pub struct StaticVariable {
    pub name: Identifier,
    pub global: bool,
    pub init: StaticInit,
    pub ty: Type,
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
    SignExtend {
        src: Value,
        dst: Value,
    },
    Truncate {
        src: Value,
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
    Constant(Constant),
    Var(Identifier),
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Constant(c) => c.ty(),
            Value::Var(i) => get_symbols().get(i).unwrap().ty.clone(),
        }
    }

    pub fn const_ty(&self) -> Option<ConstantType> {
        match self {
            Value::Constant(c) => Some(c.discriminant()),
            Value::Var(i) => get_symbols().get(i).unwrap().ty.to_constant(),
        }
    }

    pub fn asm_type(&self) -> AssemblyType {
        match self.ty() {
            Type::Int => AssemblyType::Longword,
            Type::Long => AssemblyType::Quadword,
            Type::Function(_) => unimplemented!(),
        }
    }
}
