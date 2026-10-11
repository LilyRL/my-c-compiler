use super::AssemblyType;
use crate::core::{Symbol, Type};
use crate::tacky;

impl Type {
    pub fn to_asm_type(&self) -> Option<AssemblyType> {
        match self {
            Type::Int | Type::UInt => Some(AssemblyType::Longword),
            Type::Long | Type::ULong => Some(AssemblyType::Quadword),
            Type::Float => Some(AssemblyType::Single),
            Type::Double => Some(AssemblyType::Double),
            Type::Function(_) => None,
        }
    }
}

impl Symbol {
    pub fn asm_type(&self) -> Option<AssemblyType> {
        self.ty.to_asm_type()
    }
}

impl tacky::Value {
    pub fn asm_type(&self, symbols: &crate::core::Symbols) -> AssemblyType {
        self.ty(symbols)
            .to_asm_type()
            .expect(".asm_type() called on a function")
    }
}
