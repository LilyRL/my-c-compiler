use ordered_float::OrderedFloat;
use strum::{EnumDiscriminants, EnumIs, IntoDiscriminant};

use crate::core::{StaticInit, Type};

#[derive(Debug, Clone, Copy, EnumDiscriminants, EnumIs)]
#[strum_discriminants(name(ConstantType))]
pub enum Constant {
    Int(i32),
    UInt(u32),
    Long(i64),
    ULong(u64),
    Double(f64),
    Float(f32),
}

impl std::cmp::PartialEq for Constant {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(i1), Self::Int(i2)) => i1 == i2,
            (Self::UInt(u1), Self::UInt(u2)) => u1 == u2,
            (Self::Long(l1), Self::Long(l2)) => l1 == l2,
            (Self::ULong(ul1), Self::ULong(ul2)) => ul1 == ul2,
            (Self::Double(d1), Self::Double(d2)) => d1.to_bits() == d2.to_bits(),
            (Self::Float(f1), Self::Float(f2)) => f1.to_bits() == f2.to_bits(),
            _ => false,
        }
    }
}

impl std::cmp::PartialOrd for Constant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Int(i1), Self::Int(i2)) => i1.partial_cmp(i2),
            (Self::UInt(u1), Self::UInt(u2)) => u1.partial_cmp(u2),
            (Self::Long(l1), Self::Long(l2)) => l1.partial_cmp(l2),
            (Self::ULong(ul1), Self::ULong(ul2)) => ul1.partial_cmp(ul2),
            (Self::Double(d1), Self::Double(d2)) => d1.to_bits().partial_cmp(&d2.to_bits()),
            (Self::Float(f1), Self::Float(f2)) => f1.to_bits().partial_cmp(&f2.to_bits()),
            _ => None,
        }
    }
}

impl std::cmp::Eq for Constant {}

impl std::hash::Hash for Constant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Int(i) => i.hash(state),
            Self::UInt(u) => u.hash(state),
            Self::Long(l) => l.hash(state),
            Self::ULong(ul) => ul.hash(state),
            Self::Double(d) => d.to_bits().hash(state),
            Self::Float(f) => f.to_bits().hash(state),
        }
    }
}

impl ConstantType {
    pub fn is_integer(&self) -> bool {
        match self {
            ConstantType::Int | ConstantType::Long | ConstantType::UInt | ConstantType::ULong => {
                true
            }
            ConstantType::Double | ConstantType::Float => false,
        }
    }
}

impl Constant {
    pub fn const_type(&self) -> ConstantType {
        self.discriminant()
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Constant::Int(i) => *i == 0,
            Constant::Long(l) => *l == 0,
            Constant::UInt(u) => *u == 0,
            Constant::ULong(ul) => *ul == 0,
            Constant::Double(d) => *d == 0.0,
            Constant::Float(f) => *f == 0.0,
        }
    }

    pub fn from_int(i: i32, ty: ConstantType) -> Self {
        match ty {
            ConstantType::Int => Constant::Int(i),
            ConstantType::Long => Constant::Long(i as i64),
            ConstantType::UInt => Constant::UInt(i as u32),
            ConstantType::ULong => Constant::ULong(i as u64),
            ConstantType::Double => Constant::Double(i as f64),
            ConstantType::Float => Constant::Float(i as f32),
        }
    }

    pub fn to_common_pair(self, other: Self) -> (Self, Self) {
        if self.const_type() == other.const_type() {
            (self, other)
        } else if self.const_type().is_integer() && other.const_type().is_integer() {
            (self.cast_long(), other.cast_long())
        } else if self.is_double() || other.is_double() {
            (self.cast_double(), self.cast_double())
        } else {
            (self.cast_float(), other.cast_float())
        }
    }

    pub fn cast_long(self) -> Self {
        self.cast(ConstantType::Long)
    }

    pub fn cast_float(self) -> Self {
        self.cast(ConstantType::Float)
    }

    pub fn cast_double(self) -> Self {
        self.cast(ConstantType::Double)
    }

    pub fn cast(self, const_ty: ConstantType) -> Constant {
        match (self, const_ty) {
            (Constant::Int(i), ConstantType::Int) => Constant::Int(i),
            (Constant::Int(i), ConstantType::Long) => Constant::Long(i as i64),
            (Constant::Int(i), ConstantType::UInt) => Constant::UInt(i as u32),
            (Constant::Int(i), ConstantType::ULong) => Constant::ULong(i as u64),
            (Constant::Int(u), ConstantType::Double) => Constant::Double(u as f64),
            (Constant::Int(u), ConstantType::Float) => Constant::Float(u as f32),

            (Constant::UInt(u), ConstantType::Int) => Constant::Int(u as i32),
            (Constant::UInt(u), ConstantType::Long) => Constant::Long(u as i64),
            (Constant::UInt(u), ConstantType::UInt) => Constant::UInt(u),
            (Constant::UInt(u), ConstantType::ULong) => Constant::ULong(u as u64),
            (Constant::UInt(u), ConstantType::Double) => Constant::Double(u as f64),
            (Constant::UInt(u), ConstantType::Float) => Constant::Float(u as f32),

            (Constant::Long(l), ConstantType::Int) => Constant::Int(l as i32),
            (Constant::Long(l), ConstantType::Long) => Constant::Long(l),
            (Constant::Long(l), ConstantType::UInt) => Constant::UInt(l as u32),
            (Constant::Long(l), ConstantType::ULong) => Constant::ULong(l as u64),
            (Constant::Long(l), ConstantType::Double) => Constant::Double(l as f64),
            (Constant::Long(l), ConstantType::Float) => Constant::Float(l as f32),

            (Constant::ULong(ul), ConstantType::Int) => Constant::Int(ul as i32),
            (Constant::ULong(ul), ConstantType::Long) => Constant::Long(ul as i64),
            (Constant::ULong(ul), ConstantType::UInt) => Constant::UInt(ul as u32),
            (Constant::ULong(ul), ConstantType::ULong) => Constant::ULong(ul),
            (Constant::ULong(ul), ConstantType::Double) => Constant::Double(ul as f64),
            (Constant::ULong(ul), ConstantType::Float) => Constant::Float(ul as f32),

            (Constant::Double(d), ConstantType::Int) => Constant::Int(d as i32),
            (Constant::Double(d), ConstantType::Long) => Constant::Long(d as i64),
            (Constant::Double(d), ConstantType::UInt) => Constant::UInt(d as u32),
            (Constant::Double(d), ConstantType::ULong) => Constant::ULong(d as u64),
            (Constant::Double(d), ConstantType::Double) => Constant::Double(d),
            (Constant::Double(d), ConstantType::Float) => Constant::Float(d as f32),

            (Constant::Float(f), ConstantType::Int) => Constant::Int(f as i32),
            (Constant::Float(f), ConstantType::Long) => Constant::Long(f as i64),
            (Constant::Float(f), ConstantType::UInt) => Constant::UInt(f as u32),
            (Constant::Float(f), ConstantType::ULong) => Constant::ULong(f as u64),
            (Constant::Float(f), ConstantType::Double) => Constant::Double(f as f64),
            (Constant::Float(f), ConstantType::Float) => Constant::Float(f),
        }
    }

    pub fn to_static_init(self) -> StaticInit {
        match self {
            Constant::Int(i) => StaticInit::Int(i),
            Constant::UInt(u) => StaticInit::UInt(u),
            Constant::Long(l) => StaticInit::Long(l),
            Constant::ULong(ul) => StaticInit::ULong(ul),
            Constant::Double(d) => StaticInit::Double(d),
            Constant::Float(f) => StaticInit::Float(f),
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Constant::Int(_) => Type::Int,
            Constant::Long(_) => Type::Long,
            Constant::UInt(_) => Type::UInt,
            Constant::ULong(_) => Type::ULong,
            Constant::Double(_) => Type::Double,
            Constant::Float(_) => Type::Float,
        }
    }
}
