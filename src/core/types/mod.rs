use std::fmt::{self, Display};

use strum::{EnumIs, EnumTryAs};

use crate::core::{identifier::Identifier, span::Span, symbols::StaticInit};

pub use constant::{Constant, ConstantType};
mod constant;

#[derive(Clone, PartialEq, EnumIs, Debug, EnumTryAs)]
pub enum Type {
    Int,
    Long,
    UInt,
    ULong,
    Float,
    Double,
    Function(Box<FunctionType>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Long => write!(f, "long"),
            Type::UInt => write!(f, "unsigned int"),
            Type::ULong => write!(f, "unsigned long"),
            Type::Float => write!(f, "float"),
            Type::Double => write!(f, "double"),
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
    pub fn is_signed_int(&self) -> bool {
        match self {
            Type::Int | Type::Long => true,
            Type::UInt | Type::ULong => false,
            Type::Float | Type::Double => false,
            Type::Function(_) => true,
        }
    }

    pub fn size_bytes(&self) -> u32 {
        match self {
            Type::Int | Type::UInt => 4,
            Type::Long | Type::ULong => 8,
            Type::Float => 4,
            Type::Double => 8,
            Type::Function(_) => 8,
        }
    }

    pub fn alignment(&self) -> u32 {
        match self {
            Type::Int | Type::UInt => 4,
            Type::Long | Type::ULong => 8,
            Type::Float => 4,
            Type::Double => 8,
            Type::Function(_) => 8,
        }
    }

    pub fn to_constant(&self) -> Option<ConstantType> {
        match self {
            Type::Int => Some(ConstantType::Int),
            Type::Long => Some(ConstantType::Long),
            Type::UInt => Some(ConstantType::UInt),
            Type::ULong => Some(ConstantType::ULong),
            Type::Float => Some(ConstantType::Float),
            Type::Double => Some(ConstantType::Double),
            Type::Function(_) => None,
        }
    }

    pub fn is_floating_point(&self) -> bool {
        self.is_float() || self.is_double()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub defined: bool,
    pub return_type: Type,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Specifier {
    Int,
    Long,
    Unsigned,
    Signed,
    Static,
    Extern,
    Float,
    Double,
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
            Self::Float(fl) => write!(f, "{}", fl),
            Self::Double(d) => write!(f, "{}", d),
        }
    }
}

impl Display for StaticInit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::Long(l) => write!(f, "{l}"),
            Self::UInt(u) => write!(f, "{u}"),
            Self::ULong(ul) => write!(f, "{ul}"),
            Self::Float(float) => write!(f, "{float}"),
            Self::Double(double) => write!(f, "{double}"),
        }
    }
}
