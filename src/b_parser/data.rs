use crate::{
    c_ir::{Instruction, Value},
    ir,
};

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub block: Statement,
}

#[derive(Debug, Clone)]
pub struct Identifier(pub String);

impl Identifier {
    pub fn rand() -> Self {
        Self((0..6).map(|_| rand::random_range('a'..'z')).collect())
    }
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
}

#[derive(Debug)]
pub enum Expression {
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
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    Complement,
    Negate,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl BinaryOperator {
    pub fn precedence(self) -> u32 {
        match self {
            Self::Add | Self::Subtract => 45,
            Self::Multiply | Self::Divide | Self::Remainder => 50,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
        }
    }

    pub fn lower(self) -> crate::codegen::BinaryOperator {
        match self {
            Self::Add => crate::codegen::BinaryOperator::Add,
            Self::Subtract => crate::codegen::BinaryOperator::Sub,
            Self::Multiply => crate::codegen::BinaryOperator::Mult,
            _ => unimplemented!(),
        }
    }

    pub fn not_divide_or_remainder(self) -> bool {
        match self {
            Self::Divide | Self::Remainder => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
pub enum Constant {
    Int(i32),
}

impl Program {
    pub fn lower(self) -> ir::Program {
        ir::Program(self.0.lower())
    }
}

impl FunctionDefinition {
    pub fn lower(self) -> ir::FunctionDefinition {
        let mut instructions = Vec::new();

        for statement in [self.block] {
            statement.lower(&mut instructions);
        }

        ir::FunctionDefinition {
            name: self.name,
            body: instructions,
        }
    }
}

impl Statement {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Return(c) => {
                let dst = c.lower(instructions);
                instructions.push(Instruction::Return(dst));
            }
        }
    }
}

impl Expression {
    pub fn lower(self, instructions: &mut Vec<Instruction>) -> Value {
        match self {
            Self::Constant(c) => c.lower(),
            Self::Unary { operator, expr } => {
                let src = expr.lower(instructions);
                let dst = Value::Var(Identifier::rand());

                instructions.push(Instruction::Unary {
                    operator,
                    src,
                    dst: dst.clone(),
                });
                dst
            }
            Self::Binary { operator, lhs, rhs } => {
                let lhs = lhs.lower(instructions);
                let rhs = rhs.lower(instructions);
                let dst = Value::Var(Identifier::rand());

                instructions.push(Instruction::Binary {
                    operator,
                    lhs,
                    rhs,
                    dst: dst.clone(),
                });
                return dst;
            }
        }
    }
}

impl Constant {
    pub fn lower(self) -> ir::Value {
        match self {
            Constant::Int(i) => ir::Value::Constant(i),
        }
    }
}
