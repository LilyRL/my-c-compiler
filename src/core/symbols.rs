use std::collections::HashMap;

use strum::EnumIs;

use crate::core::{
    identifier::Identifier,
    span::Span,
    types::{Constant, ConstantType, Type},
};

pub type Symbols = HashMap<Identifier, Symbol>;

#[derive(Debug)]
pub struct Symbol {
    pub attributes: IdentifierAttributes,
    pub ty: Type,
    pub declared_at: Span,
}

impl Symbol {
    pub fn temporary(ty: Type) -> Self {
        Self {
            attributes: IdentifierAttributes::Local,
            ty,
            declared_at: 0..0,
        }
    }
}

#[derive(Clone, Debug, EnumIs)]
pub enum IdentifierAttributes {
    Function { defined: bool, global: bool },
    Static { init: InitialValue, global: bool },
    Local,
}

impl IdentifierAttributes {
    pub fn global(&self) -> bool {
        match self {
            IdentifierAttributes::Function { global, .. } => *global,
            IdentifierAttributes::Static { global, .. } => *global,
            IdentifierAttributes::Local => false,
        }
    }

    pub fn defined(&self) -> bool {
        match self {
            IdentifierAttributes::Function { defined, .. } => *defined,
            IdentifierAttributes::Static { .. } => true,
            IdentifierAttributes::Local => true,
        }
    }

    pub fn initial_value(&self) -> Option<InitialValue> {
        match self {
            IdentifierAttributes::Function { .. } => None,
            IdentifierAttributes::Static { init, .. } => Some(init.clone()),
            IdentifierAttributes::Local => None,
        }
    }
}

#[derive(Clone, Debug, EnumIs)]
pub enum InitialValue {
    Tentitive,
    Constant(StaticInit),
    None,
}

#[derive(Clone, Debug, Copy, PartialEq, PartialOrd)]
pub enum StaticInit {
    Int(i32),
    UInt(u32),
    Long(i64),
    ULong(u64),
    Float(f32),
    Double(f64),
}

impl Eq for StaticInit {}

impl Ord for StaticInit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bits().cmp(&other.bits())
    }
}

impl StaticInit {
    pub fn size_bytes(&self) -> usize {
        match self {
            StaticInit::Int(_) => 4,
            StaticInit::Long(_) => 8,
            StaticInit::UInt(_) => 4,
            StaticInit::ULong(_) => 8,
            StaticInit::Float(_) => 4,
            StaticInit::Double(_) => 8,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            StaticInit::Int(i) => *i == 0,
            StaticInit::Long(l) => *l == 0,
            StaticInit::UInt(u) => *u == 0,
            StaticInit::ULong(ul) => *ul == 0,
            StaticInit::Float(f) => *f == 0.0,
            StaticInit::Double(d) => *d == 0.0,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, StaticInit::Float(_) | StaticInit::Double(_))
    }

    pub fn cast(self, ty: ConstantType) -> Self {
        self.to_constant().cast(ty).to_static_init()
    }

    pub fn bits(&self) -> u64 {
        match self {
            StaticInit::Int(i) => *i as u64,
            StaticInit::Long(l) => *l as u64,
            StaticInit::UInt(u) => *u as u64,
            StaticInit::ULong(ul) => *ul,
            StaticInit::Float(f) => f.to_bits() as u64,
            StaticInit::Double(d) => d.to_bits(),
        }
    }

    pub fn from_constant(constant: Constant) -> Self {
        constant.to_static_init()
    }

    pub fn to_constant(self) -> Constant {
        match self {
            StaticInit::Int(i) => Constant::Int(i),
            StaticInit::UInt(u) => Constant::UInt(u),
            StaticInit::Long(l) => Constant::Long(l),
            StaticInit::ULong(ul) => Constant::ULong(ul),
            StaticInit::Float(f) => Constant::Float(f),
            StaticInit::Double(d) => Constant::Double(d),
        }
    }
}
