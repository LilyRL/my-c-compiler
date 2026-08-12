use std::fmt::Display;

use crate::b_parser::BinaryOperator;
use crate::codegen;
use crate::d_codegen::{Operand, Register};
use crate::parser::{Identifier, UnaryOperator};

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub body: Vec<Instruction>,
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
}

#[derive(Clone, Debug)]
pub enum Value {
    Constant(i32),
    Var(Identifier),
}

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
                    operator,
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
            Self::Return(value) => write!(f, "return {};", value),
            Self::Unary { operator, src, dst } => {
                let op_str = match operator {
                    UnaryOperator::Complement => "~",
                    UnaryOperator::Negate => "-",
                };
                write!(f, "{} = {}{};", dst, op_str, src)
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let op_str = operator.to_str();
                write!(f, "{} = {} {} {};", dst, lhs, op_str, rhs)
            }
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
            write!(f, "    {}\n", instruction)?;
        }
        write!(f, "}}")
    }
}
