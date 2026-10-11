use strum::EnumIs;

use crate::core::{Identifier, StaticInit};

#[derive(Debug)]
pub struct Program(pub Vec<TopLevel>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIs)]
pub enum AssemblyType {
    Longword,
    Quadword,
    Single,
    Double,
}

impl AssemblyType {
    pub fn size_bytes(self) -> u32 {
        match self {
            Self::Longword | Self::Single => 4,
            Self::Quadword | Self::Double => 8,
        }
    }

    pub fn alignment(self) -> i32 {
        self.size_bytes() as i32
    }

    pub(crate) fn is_float(&self) -> bool {
        matches!(self, Self::Single | Self::Double)
    }
}

#[derive(Debug)]
pub enum TopLevel {
    F(FunctionDefinition),
    V(StaticVariable),
    C(StaticConstant),
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StaticConstant {
    pub name: Identifier,
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
    /// sign extend
    Movsx {
        src: Operand,
        dst: Operand,
    },
    /// zero extend
    Movzx {
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
    ZeroOut(AssemblyType, Operand),
    // cmp for integers, ucomis* for floats
    Cmp(AssemblyType, Operand, Operand),
    // signed integers
    Idiv(AssemblyType, Operand),
    // unsigned integers
    Div(AssemblyType, Operand),
    Cdq(AssemblyType),
    Jump(Identifier),
    JumpCC(CondCode, Identifier),
    SetCC(CondCode, Operand),
    Label(Identifier),
    Push(Operand),
    Call(Identifier),
    Comment(&'static str),
    // either cvtt(ss|sd|si)2(ss|sd|si)(l|q) or a noop
    Convert {
        src: Operand,
        src_type: AssemblyType,
        dst: Operand,
        dst_type: AssemblyType,
    },
    TruncateFloat {
        ty: AssemblyType,
        src: Operand,
        dst: Operand,
    },
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
    /// set when equal
    Eq,
    /// set when not equal
    Ne,
    /// set when signed & a > b
    Gt,
    /// set when signed & a >= b
    Ge,
    /// set when signed & a < b
    Lt,
    /// set when signed & a <= b
    Le,
    /// set when unsigned & a > b
    A,
    /// set when unsigned & a >= b
    Ae,
    /// set when unsigned & a < b
    B,
    /// set when unsigned & a <= b
    Be,
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
            Self::A => "a",
            Self::Ae => "ae",
            Self::B => "b",
            Self::Be => "be",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    R12,
    R13,
    R14,
    R15,
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
    StackPointer,
}

#[derive(Debug, Clone, EnumIs)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    FloatDiv,
    Sar, // arithmetic shift, used for signed values
    Sal, // "
    Shr, // logical shift, used for unsigned values
    Shl, // "
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

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    // floating point numbers cannot be used as immidiate values
    Imm(i64),
    Reg(Register),
    Pseudo(Identifier),
    Stack(i32),
    Data(String),
    Constant(Identifier),
}

impl Register {
    /// the number in the name of a xmm register, if this is one
    pub fn xmm_number(&self) -> Option<u8> {
        Some(match self {
            Self::Xmm0 => 0,
            Self::Xmm1 => 1,
            Self::Xmm2 => 2,
            Self::Xmm3 => 3,
            Self::Xmm4 => 4,
            Self::Xmm5 => 5,
            Self::Xmm6 => 6,
            Self::Xmm7 => 7,
            Self::Xmm8 => 8,
            Self::Xmm9 => 9,
            Self::Xmm10 => 10,
            Self::Xmm11 => 11,
            Self::Xmm12 => 12,
            Self::Xmm13 => 13,
            Self::Xmm14 => 14,
            Self::Xmm15 => 15,
            _ => return None,
        })
    }
}

impl BinaryOperator {
    pub fn int_op_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "imul",
            Self::Sal => "sal",
            Self::Sar => "sar",
            Self::Shl => "shl",
            Self::Shr => "shr",
            Self::BitwiseAnd => "and",
            Self::BitwiseXor => "xor",
            Self::BitwiseOr => "or",
            Self::NotEqual
            | Self::GreaterThan
            | Self::GreaterEqual
            | Self::LessThan
            | Self::LessEqual
            | Self::FloatDiv => unimplemented!(),
        }
    }

    pub fn float_op_str(&self) -> Option<&'static str> {
        match self {
            Self::Add => Some("add"),
            Self::Sub => Some("sub"),
            Self::Mul => Some("mul"),
            Self::FloatDiv => Some("div"),
            _ => None,
        }
    }

    pub fn src_size(&self) -> Option<u32> {
        if self.is_shift() { Some(1) } else { None }
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
        matches!(self, Self::Shr | Self::Shl | Self::Sar | Self::Sal)
    }

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
        matches!(self, Self::Stack(_) | Self::Data(_) | Self::Constant(_))
    }

    pub fn is_register(&self) -> bool {
        matches!(self, Self::Reg(_))
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Imm(_))
    }
}
