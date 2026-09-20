use std::collections::HashMap;

use strum::EnumIs;

use crate::{analysis::StaticInit, parser::Identifier};

#[derive(Debug)]
pub struct Program(pub Vec<TopLevel>);

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIs)]
pub enum AssemblyType {
    Longword = 32,
    Quadword = 64,
}

impl AssemblyType {
    pub fn size_bytes(self) -> u32 {
        (self as u32) / 8
    }

    pub fn alignment(self) -> i32 {
        self.size_bytes() as i32
    }
}

pub type AsmSymbols = HashMap<Identifier, AsmSymbol>;
sge_global::global!(AsmSymbols, asm_symbols);
pub fn set_asm_symbol_table(symbols: AsmSymbols) {
    set_asm_symbols(symbols);
}

pub enum AsmSymbol {
    Object { ty: AssemblyType, is_static: bool },
    Function { defined: bool },
}

impl AsmSymbol {
    pub fn is_static(&self) -> bool {
        match self {
            Self::Object { is_static, .. } => *is_static,
            Self::Function { .. } => false,
        }
    }

    pub fn ty(&self) -> Option<AssemblyType> {
        match self {
            Self::Object { ty, .. } => Some(*ty),
            Self::Function { .. } => None,
        }
    }

    pub fn defined(&self) -> bool {
        match self {
            Self::Object { .. } => true,
            Self::Function { defined } => *defined,
        }
    }
}

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
    pub init: StaticInit,
    pub alignment: u32,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov {
        ty: AssemblyType,
        src: Operand,
        dst: Operand,
    },
    Movsx {
        src: Operand,
        dst: Operand,
    },
    Unary {
        ty: AssemblyType,
        operator: UnaryOperator,
        operand: Operand,
    },
    Binary {
        ty: AssemblyType,
        operator: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    Cmp(AssemblyType, Operand, Operand),
    Idiv(AssemblyType, Operand),
    Cdq(AssemblyType),
    Jump(Identifier),
    JumpCC(CondCode, Identifier),
    SetCC(CondCode, Operand),
    Label(Identifier),
    Push(Operand),
    Call(Identifier),
    Comment(&'static str),
    Ret,
}

impl Instruction {
    pub fn deallocate_stack(bytes: u32) -> Self {
        Self::Binary {
            ty: AssemblyType::Quadword,
            operator: BinaryOperator::Add,
            src: Operand::Imm(bytes as i64),
            dst: Operand::Reg(Register::StackPointer),
        }
    }

    pub fn allocate_stack(bytes: u32) -> Self {
        Self::Binary {
            ty: AssemblyType::Quadword,
            operator: BinaryOperator::Sub,
            src: Operand::Imm(bytes as i64),
            dst: Operand::Reg(Register::StackPointer),
        }
    }
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
    StackPointer,
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
    Imm(i64),
    Reg(Register),
    Pseudo(Identifier),
    Stack(i32),
    Data(Identifier),
}

impl BinaryOperator {
    pub fn op_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "imul",
            Self::LeftShift => "sal",
            Self::RightShift => "sar",
            Self::BitwiseAnd => "and",
            Self::BitwiseXor => "xor",
            Self::BitwiseOr => "or",
            Self::NotEqual
            | Self::GreaterThan
            | Self::GreaterEqual
            | Self::LessThan
            | Self::LessEqual => unimplemented!(),
        }
    }

    pub fn src_size(&self) -> Option<u32> {
        match self {
            Self::LeftShift | Self::RightShift => Some(1),
            _ => None,
        }
    }

    pub fn cant_have_double_memory(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::BitwiseAnd | Self::BitwiseXor | Self::BitwiseOr
        )
    }

    pub fn cant_have_large_imm(&self) -> bool {
        matches!(
            self,
            Self::Add
                | Self::Sub
                | Self::Mul
                | Self::BitwiseOr
                | Self::BitwiseAnd
                | Self::BitwiseXor
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
            Self::BitwiseNot => "not",
            Self::Negate => "neg",
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
