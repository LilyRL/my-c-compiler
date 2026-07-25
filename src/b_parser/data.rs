use crate::codegen;

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub block: Statement,
}

#[derive(Debug)]
pub struct Identifier(pub String);

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
}

#[derive(Debug)]
pub enum Expression {
    Constant(Constant),
}

#[derive(Debug)]
pub enum Constant {
    Int(i32),
}

impl Program {
    pub fn lower(self) -> codegen::Program {
        codegen::Program(self.0.lower())
    }
}

impl FunctionDefinition {
    pub fn lower(self) -> codegen::FunctionDefinition {
        codegen::FunctionDefinition {
            name: self.name,
            instructions: self.block.lower(),
        }
    }
}

impl Statement {
    pub fn lower(self) -> Vec<codegen::Instruction> {
        match self {
            Self::Return(c) => vec![
                codegen::Instruction::Mov {
                    src: c.lower(),
                    dst: codegen::Operand::Register,
                },
                codegen::Instruction::Ret,
            ],
        }
    }
}

impl Expression {
    pub fn lower(self) -> codegen::Operand {
        match self {
            Self::Constant(c) => c.lower(),
        }
    }
}

impl Constant {
    pub fn lower(self) -> codegen::Operand {
        match self {
            Constant::Int(i) => codegen::Operand::Imm(i),
        }
    }
}
