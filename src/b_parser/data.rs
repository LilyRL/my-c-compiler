use crate::{
    a_lexer::Token,
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
    fn inner() -> String {
        (0..4).map(|_| rand::random_range('a'..'z')).collect()
    }

    pub fn new(name: &str) -> Self {
        Self(format!(".{name}_{}", Self::inner()))
    }

    pub fn rand() -> Self {
        Self(Self::inner())
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
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    And,
    Or,
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

impl UnaryOperator {
    pub fn lower(self) -> ir::UnaryOperator {
        match self {
            Self::BitwiseNot => ir::UnaryOperator::BitwiseNot,
            Self::Negate => ir::UnaryOperator::Negate,
            Self::Not => ir::UnaryOperator::Not,
        }
    }
}

impl BinaryOperator {
    pub fn precedence(self) -> u32 {
        match self {
            Self::Or => 15,
            Self::And => 20,
            Self::BitwiseOr => 25,
            Self::BitwiseXor => 30,
            Self::BitwiseAnd => 35,
            Self::Equal | Self::NotEqual => 38,
            Self::GreaterThan | Self::GreaterEqual | Self::LessThan | Self::LessEqual => 39,
            Self::LeftShift
            | Self::RightShift
            | Self::LogicalLeftShift
            | Self::LogicalRightShift => 40,
            Self::Add | Self::Subtract => 45,
            Self::Multiply | Self::Divide | Self::Remainder => 50,
        }
    }

    pub fn can_be_lowered(self) -> bool {
        match self {
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Remainder
            | Self::LeftShift
            | Self::RightShift
            | Self::LogicalLeftShift
            | Self::LogicalRightShift
            | Self::BitwiseAnd
            | Self::BitwiseXor
            | Self::BitwiseOr
            | Self::Equal
            | Self::NotEqual
            | Self::LessThan
            | Self::LessEqual
            | Self::GreaterThan
            | Self::GreaterEqual => true,
            Self::And | Self::Or => false,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "rem",
            Self::LeftShift => "shl",
            Self::RightShift => "shr",
            Self::LogicalLeftShift => "shl",
            Self::LogicalRightShift => "shr",
            Self::BitwiseAnd => "and",
            Self::BitwiseXor => "xor",
            Self::BitwiseOr => "or",
            Self::And => "and",
            Self::Or => "or",
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::LessThan => "lt",
            Self::LessEqual => "le",
            Self::GreaterThan => "gt",
            Self::GreaterEqual => "ge",
        }
    }

    pub fn lower(self) -> ir::BinaryOperator {
        match self {
            Self::Add => ir::BinaryOperator::Add,
            Self::Subtract => ir::BinaryOperator::Subtract,
            Self::Multiply => ir::BinaryOperator::Multiply,
            Self::Divide => ir::BinaryOperator::Divide,
            Self::Remainder => ir::BinaryOperator::Remainder,
            Self::LeftShift => ir::BinaryOperator::LeftShift,
            Self::RightShift => ir::BinaryOperator::RightShift,
            Self::LogicalLeftShift => ir::BinaryOperator::LogicalLeftShift,
            Self::LogicalRightShift => ir::BinaryOperator::LogicalRightShift,
            Self::BitwiseAnd => ir::BinaryOperator::BitwiseAnd,
            Self::BitwiseXor => ir::BinaryOperator::BitwiseXor,
            Self::BitwiseOr => ir::BinaryOperator::BitwiseOr,
            Self::NotEqual => ir::BinaryOperator::NotEqual,
            Self::LessThan => ir::BinaryOperator::LessThan,
            Self::LessEqual => ir::BinaryOperator::LessEqual,
            Self::GreaterThan => ir::BinaryOperator::GreaterThan,
            Self::GreaterEqual => ir::BinaryOperator::GreaterEqual,
            Self::Equal => ir::BinaryOperator::Equal,
            Self::And | Self::Or => unimplemented!(),
        }
    }

    pub fn from_token(token: Token) -> Option<BinaryOperator> {
        match token {
            Token::Plus => Some(BinaryOperator::Add),
            Token::Hyphen => Some(BinaryOperator::Subtract),
            Token::Asterisk => Some(BinaryOperator::Multiply),
            Token::Slash => Some(BinaryOperator::Divide),
            Token::Percent => Some(BinaryOperator::Remainder),
            Token::Ampersand => Some(BinaryOperator::BitwiseAnd),
            Token::Caret => Some(BinaryOperator::BitwiseXor),
            Token::Pipe => Some(BinaryOperator::BitwiseOr),
            Token::LeftShift => Some(BinaryOperator::LeftShift),
            Token::RightShift => Some(BinaryOperator::RightShift),
            Token::LogicalLeftShift => Some(BinaryOperator::LogicalLeftShift),
            Token::LogicalRightShift => Some(BinaryOperator::LogicalRightShift),
            Token::LogicalAnd => Some(BinaryOperator::And),
            Token::LogicalOr => Some(BinaryOperator::Or),
            Token::Equal => Some(BinaryOperator::Equal),
            Token::NotEqual => Some(BinaryOperator::NotEqual),
            Token::LessThan => Some(BinaryOperator::LessThan),
            Token::LessEqual => Some(BinaryOperator::LessEqual),
            Token::GreaterThan => Some(BinaryOperator::GreaterThan),
            Token::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            _ => None,
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
                let dst = Value::Var(Identifier::new("tmp"));

                instructions.push(Instruction::Unary {
                    operator: operator.lower(),
                    src,
                    dst: dst.clone(),
                });
                dst
            }
            Self::Binary { operator, lhs, rhs } if operator.can_be_lowered() => {
                let lhs = lhs.lower(instructions);
                let rhs = rhs.lower(instructions);
                let dst = Value::Var(Identifier::new(operator.name()));

                instructions.push(Instruction::Binary {
                    operator: operator.lower(),
                    lhs,
                    rhs,
                    dst: dst.clone(),
                });
                return dst;
            }
            Self::Binary { operator, lhs, rhs } => match operator {
                BinaryOperator::And => {
                    let dst = Value::Var(Identifier::new("and_result"));
                    let false_label = Identifier::new("false");
                    let end_label = Identifier::new("end");

                    let lhs = lhs.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: lhs,
                        target: false_label.clone(),
                    });

                    let rhs = rhs.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: rhs,
                        target: false_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(1),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(false_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(0),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                BinaryOperator::Or => {
                    let dst = Value::Var(Identifier::new("or_result"));
                    let true_label = Identifier::new("true");
                    let end_label = Identifier::new("end");

                    let lhs = lhs.lower(instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: lhs,
                        target: true_label.clone(),
                    });

                    let rhs = rhs.lower(instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: rhs,
                        target: true_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(0),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(true_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(1),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                _ => unreachable!(),
            },
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
