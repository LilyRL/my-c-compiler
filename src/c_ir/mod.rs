pub use data::*;
mod data;

use std::fmt::Display;

use crate::codegen;
use crate::codegen::{Operand, Register};

impl Value {
    pub fn lower(self) -> codegen::Operand {
        match self {
            Self::Constant(int) => codegen::Operand::Imm(int),
            Self::Var(ident) => codegen::Operand::Pseudo(ident),
        }
    }
}

impl Instruction {
    pub fn lower(self, instructions: &mut Vec<codegen::Instruction>) {
        match self {
            Self::Return(val) => {
                instructions.push(codegen::Instruction::Mov {
                    src: val.lower(),
                    dst: codegen::Operand::Reg(codegen::Register::Ax),
                });
                instructions.push(codegen::Instruction::Ret);
            }
            Self::Unary { operator, src, dst } => {
                let dst = dst.lower();
                instructions.push(codegen::Instruction::Mov {
                    src: src.lower(),
                    dst: dst.clone(),
                });
                instructions.push(codegen::Instruction::Unary {
                    operator: operator.lower(),
                    operand: dst,
                });
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let dst = dst.lower();
                let lhs = lhs.lower();
                let rhs = rhs.lower();

                if operator.not_divide_or_remainder() {
                    instructions.push(codegen::Instruction::Mov {
                        src: lhs,
                        dst: dst.clone(),
                    });
                    instructions.push(codegen::Instruction::Binary {
                        operator: operator.lower(),
                        src: rhs,
                        dst,
                    });
                } else {
                    let register = match operator {
                        BinaryOperator::Divide => Register::Ax,
                        BinaryOperator::Remainder => Register::Dx,
                        _ => unreachable!(),
                    };

                    instructions.push(codegen::Instruction::Mov {
                        src: lhs,
                        dst: Operand::Reg(Register::Ax),
                    });
                    instructions.push(codegen::Instruction::Cdq);
                    instructions.push(codegen::Instruction::Idiv(rhs));
                    instructions.push(codegen::Instruction::Mov {
                        src: Operand::Reg(register),
                        dst,
                    });
                }
            }
            _ => todo!(),
        }
    }
}

impl FunctionDefinition {
    pub fn lower(self) -> codegen::FunctionDefinition {
        let mut instructions = vec![];

        for instruction in self.body {
            instruction.lower(&mut instructions);
        }

        codegen::FunctionDefinition {
            name: self.name,
            instructions,
        }
    }
}

impl Program {
    pub fn lower(self) -> codegen::Program {
        codegen::Program(self.0.lower())
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return(value) => write!(f, "\treturn {};", value),
            Self::Unary { operator, src, dst } => {
                let op_str = operator.to_str();
                write!(f, "\t{} = {}{};", dst, op_str, src)
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let op_str = operator.to_str();
                write!(f, "\t{} = {} {} {};", dst, lhs, op_str, rhs)
            }
            Self::Copy { src, dst } => write!(f, "\t{} = {};", dst, src),
            Self::Jump(target) => write!(f, "\tgoto {};", target.0),
            Self::JumpIfZero { condition, target } => {
                write!(f, "\tjz {} => {}", condition, target.0)
            }
            Self::JumpNotZero { condition, target } => {
                write!(f, "\tjnz {} => {}", condition, target.0)
            }
            Self::Label(label) => write!(f, "{}:", label.0),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(c) => write!(f, "{}", c),
            Self::Var(v) => write!(f, "{}", v.0),
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for FunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {}() {{\n", self.name.0)?;
        for instruction in &self.body {
            write!(f, "{}\n", instruction)?;
        }
        write!(f, "}}")
    }
}

impl UnaryOperator {
    pub fn lower(self) -> codegen::UnaryOperator {
        match self {
            Self::Negate => codegen::UnaryOperator::Negate,
            Self::BitwiseNot => codegen::UnaryOperator::BitwiseNot,
            Self::Not => codegen::UnaryOperator::Not,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Self::BitwiseNot => "~",
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

impl BinaryOperator {
    pub fn not_divide_or_remainder(self) -> bool {
        match self {
            Self::Divide | Self::Remainder => false,
            _ => true,
        }
    }

    pub fn lower(self) -> codegen::BinaryOperator {
        match self {
            Self::Add => codegen::BinaryOperator::Add,
            Self::Subtract => codegen::BinaryOperator::Sub,
            Self::Multiply => codegen::BinaryOperator::Mul,
            Self::LeftShift => codegen::BinaryOperator::LeftShift,
            Self::RightShift => codegen::BinaryOperator::RightShift,
            Self::LogicalLeftShift => codegen::BinaryOperator::LogicalLeftShift,
            Self::LogicalRightShift => codegen::BinaryOperator::LogicalRightShift,
            Self::BitwiseAnd => codegen::BinaryOperator::BitwiseAnd,
            Self::BitwiseXor => codegen::BinaryOperator::BitwiseXor,
            Self::BitwiseOr => codegen::BinaryOperator::BitwiseOr,
            Self::NotEqual => codegen::BinaryOperator::NotEqual,
            Self::LessThan => codegen::BinaryOperator::LessThan,
            Self::LessEqual => codegen::BinaryOperator::LessEqual,
            Self::GreaterThan => codegen::BinaryOperator::GreaterThan,
            Self::GreaterEqual => codegen::BinaryOperator::GreaterEqual,
            _ => unimplemented!(),
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::LogicalLeftShift => "<<<",
            Self::LogicalRightShift => ">>>",
            Self::BitwiseAnd => "&",
            Self::BitwiseXor => "^",
            Self::BitwiseOr => "|",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::LessEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterEqual => ">=",
        }
    }
}
