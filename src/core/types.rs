use std::fmt::Display;

use strum::{EnumDiscriminants, EnumIs, EnumTryAs, IntoDiscriminant};

use crate::core::{identifier::Identifier, span::Span, symbols::StaticInit};

#[derive(Clone, PartialEq, EnumIs, Debug, EnumTryAs)]
pub enum Type {
    Int,
    Long,
    UInt,
    ULong,
    Function(Box<FunctionType>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Long => write!(f, "long"),
            Type::UInt => write!(f, "unsigned int"),
            Type::ULong => write!(f, "unsigned long"),
            Type::Function(func) => {
                write!(f, "{} (", func.return_type)?;

                for (i, param) in func.parameters.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }

                write!(f, ")")
            }
        }
    }
}

impl Type {
    pub fn is_signed(&self) -> bool {
        match self {
            Type::Int | Type::Long => true,
            Type::UInt | Type::ULong => false,
            Type::Function(_) => true,
        }
    }

    pub fn size_bytes(&self) -> u32 {
        match self {
            Type::Int | Type::UInt => 4,
            Type::Long | Type::ULong => 8,
            Type::Function(_) => 8,
        }
    }

    pub fn alignment(&self) -> u32 {
        match self {
            Type::Int | Type::UInt => 4,
            Type::Long | Type::ULong => 8,
            Type::Function(_) => 8,
        }
    }

    pub fn to_constant(&self) -> Option<ConstantType> {
        match self {
            Type::Int => Some(ConstantType::Int),
            Type::Long => Some(ConstantType::Long),
            Type::UInt => Some(ConstantType::UInt),
            Type::ULong => Some(ConstantType::ULong),
            Type::Function(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub defined: bool,
    pub return_type: Type,
}

#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord, Clone, Copy, EnumDiscriminants)]
#[strum_discriminants(name(ConstantType))]
pub enum Constant {
    Int(i32),
    UInt(u32),
    Long(i64),
    ULong(u64),
}

impl Constant {
    pub fn from_magnitude(magnitude: u128, ty: ConstantType) -> Self {
        match ty {
            ConstantType::Int => Constant::Int(magnitude as i32),
            ConstantType::UInt => Constant::UInt(magnitude as u32),
            ConstantType::Long => Constant::Long(magnitude as i64),
            ConstantType::ULong => Constant::ULong(magnitude as u64),
        }
    }

    pub fn i64(&self) -> i64 {
        match self {
            Constant::Int(i) => *i as i64,
            Constant::Long(l) => *l,
            Constant::UInt(u) => *u as i64,
            Constant::ULong(ul) => *ul as i64,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Constant::Int(i) => *i == 0,
            Constant::Long(l) => *l == 0,
            Constant::UInt(u) => *u == 0,
            Constant::ULong(ul) => *ul == 0,
        }
    }

    pub fn from_int(i: i32, ty: ConstantType) -> Self {
        match ty {
            ConstantType::Int => Constant::Int(i),
            ConstantType::Long => Constant::Long(i as i64),
            ConstantType::UInt => Constant::UInt(i as u32),
            ConstantType::ULong => Constant::ULong(i as u64),
        }
    }

    pub fn to_common_pair(self, other: Self) -> (Self, Self) {
        if self.discriminant() == other.discriminant() {
            (self, other)
        } else {
            (self.cast_long(), other.cast_long())
        }
    }

    pub fn cast_long(self) -> Self {
        self.cast(ConstantType::Long)
    }

    fn bits(self) -> u128 {
        match self {
            Constant::Int(i) => i as i128 as u128,
            Constant::UInt(u) => u as u128,
            Constant::Long(l) => l as i128 as u128,
            Constant::ULong(ul) => ul as u128,
        }
    }

    pub fn cast(self, const_ty: ConstantType) -> Constant {
        let bits = self.bits();

        match const_ty {
            ConstantType::Int => Constant::Int(bits as i32),
            ConstantType::UInt => Constant::UInt(bits as u32),
            ConstantType::Long => Constant::Long(bits as i64),
            ConstantType::ULong => Constant::ULong(bits as u64),
        }
    }

    pub fn to_static_init(self) -> StaticInit {
        match self {
            Constant::Int(i) => StaticInit::Int(i),
            Constant::UInt(u) => StaticInit::UInt(u),
            Constant::Long(l) => StaticInit::Long(l),
            Constant::ULong(ul) => StaticInit::ULong(ul),
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Constant::Int(_) => Type::Int,
            Constant::Long(_) => Type::Long,
            Constant::UInt(_) => Type::UInt,
            Constant::ULong(_) => Type::ULong,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Specifier {
    Int,
    Long,
    Unsigned,
    Signed,
    Static,
    Extern,
}

impl Specifier {
    pub fn storage_class(&self) -> StorageClass {
        match self {
            Specifier::Static => StorageClass::Static,
            Specifier::Extern => StorageClass::Extern,
            _ => StorageClass::None,
        }
    }
}

#[derive(Debug, EnumIs)]
pub enum StorageClass {
    Static,
    Extern,
    None,
}

#[derive(Debug)]
pub struct FunctionParameter {
    pub name: Identifier,
    pub span: Span,
    pub ty: Type,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{}", i),
            Self::Long(l) => write!(f, "{}", l),
            Self::UInt(u) => write!(f, "{}", u),
            Self::ULong(ul) => write!(f, "{}", ul),
        }
    }
}

impl Display for StaticInit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{}", i),
            Self::Long(l) => write!(f, "{}", l),
            Self::UInt(u) => write!(f, "{}", u),
            Self::ULong(ul) => write!(f, "{}", ul),
        }
    }
}
