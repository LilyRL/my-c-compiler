use std::ops::{Add, Rem, Sub};

pub use data::*;

pub mod transformations;
pub use transformations::transform;

use crate::core::{Constant, Identifier};

mod data;
mod emission;
mod lower;
mod types;

pub const R10: Operand = Operand::Reg(Register::R10);
pub const R11: Operand = Operand::Reg(Register::R11);
pub const R12: Operand = Operand::Reg(Register::R12);
pub const AX: Operand = Operand::Reg(Register::Ax);
pub const DX: Operand = Operand::Reg(Register::Dx);

// its important we dont use XMM[0-7] for scratch, as they are used for parameters
// used for float conversions
pub const XMM8: Operand = Operand::Reg(Register::Xmm8);
// used for jump if zero
pub const XMM9: Operand = Operand::Reg(Register::Xmm9);
pub const XMM10: Operand = Operand::Reg(Register::Xmm10);
pub const XMM11: Operand = Operand::Reg(Register::Xmm11);

// i64::MAX + 1 as a double
// this is the maximum value of a double that can be converted to an unsigned long in one go, with cvttsd2siq
pub const DOUBLE_TO_UINT_UPPER_BOUND: Constant = Constant::Double(9_223_372_036_854_775_808.0);
pub const FLOAT_TO_UINT_UPPER_BOUND: Constant = Constant::Float(9_223_372_036_854_775_808.0);
pub const UINT_UPPER_BOUND: Operand = Operand::Imm(9223372036854775808u64.cast_signed());

pub fn align_to<T>(n: T, to: T) -> T
where
    T: Copy + PartialEq + Default + Add<Output = T> + Sub<Output = T> + Rem<Output = T>,
{
    let rem = n % to;
    if rem == T::default() {
        n
    } else {
        n + (to - rem)
    }
}

pub fn round_up_16(bytes: u32) -> u32 {
    align_to(bytes, 16)
}

pub fn can_fit_in_i32(n: i64) -> bool {
    const MIN: i64 = i32::MIN as i64;
    const MAX: i64 = i32::MAX as i64;

    (MIN..MAX).contains(&n)
}
