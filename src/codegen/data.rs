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
    pub instructions: Vec<Instruction>,
    pub global: bool,
}

#[derive(Debug)]
pub struct StaticVariable {
    pub name: Identifier,
    pub global: bool,
    pub init: Constant,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        operator: UnaryOperator,
        operand: Operand,
    },
    Binary {
        operator: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    Cmp(Operand, Operand),
    Idiv(Operand),
    Cdq,
    Jump(Identifier),
    JumpCC(CondCode, Identifier),
    SetCC(CondCode, Operand),
    Label(Identifier),
    AllocateStack(u32),
    DeallocateStack(u32),
    Push(Operand),
    Call(Identifier),
    Comment(&'static str),
    Ret,
}

#[derive(Debug, Clone, Copy)]
pub enum CondCode {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl CondCode {
    pub fn format(self) -> &'static str {
        match self {
            Self::Eq => "e",
            Self::Ne => "ne",
            Self::Lt => "l",
            Self::Gt => "g",
            Self::Le => "le",
            Self::Ge => "ge",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Register {
    Ax,
    Cx,
    Dx,
    Di,
    Si,
    R8,
    R9,
    R10,
    R11,
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    NotEqual,
    GreaterThan,
    GreaterEqual,
    LessThan,
    LessEqual,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    BitwiseNot,
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(Identifier),
    Stack(i32),
    Data(Identifier),
}

impl BinaryOperator {
    pub fn op_str(&self) -> &'static str {
        match self {
            Self::Add => "addl",
            Self::Sub => "subl",
            Self::Mul => "imull",
            Self::LeftShift => "sall",
            Self::RightShift => "sarl",
            Self::BitwiseAnd => "andl",
            Self::BitwiseXor => "xorl",
            Self::BitwiseOr => "orl",
            Self::NotEqual
            | Self::GreaterThan
            | Self::GreaterEqual
            | Self::LessThan
            | Self::LessEqual => unimplemented!(),
        }
    }

    pub fn src_size(&self) -> u32 {
        match self {
            Self::Add
            | Self::Sub
            | Self::Mul
            | Self::BitwiseAnd
            | Self::BitwiseXor
            | Self::BitwiseOr => 4,
            Self::LeftShift | Self::RightShift => 1,
            Self::NotEqual
            | Self::GreaterThan
            | Self::GreaterEqual
            | Self::LessThan
            | Self::LessEqual => unimplemented!(),
        }
    }

    pub fn dst_size(&self) -> u32 {
        4
    }

    pub fn cant_have_double_memory(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::BitwiseAnd | Self::BitwiseXor | Self::BitwiseOr
        )
    }

    pub fn is_shift(&self) -> bool {
        matches!(self, Self::LeftShift | Self::RightShift)
    }

    /// Returns `true` if the codegen binary operator is [`Mult`].
    ///
    /// [`Mult`]: BinaryOperator::Mult
    #[must_use]
    pub fn is_mult(&self) -> bool {
        matches!(self, Self::Mul)
    }
}

impl UnaryOperator {
    pub fn op_str(&self) -> &'static str {
        match self {
            Self::BitwiseNot => "notl",
            Self::Negate => "negl",
            Self::Not => unimplemented!(),
        }
    }
}

impl Operand {
    #[must_use]
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Stack(_) | Self::Data(_))
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Imm(_))
    }
}
