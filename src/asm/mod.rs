use std::ops::{Add, Rem, Sub};

pub use data::*;

pub mod transformations;
pub use transformations::transform;

mod data;
mod emission;
mod lower;
mod types;

pub const R10: Operand = Operand::Reg(Register::R10);
pub const R11: Operand = Operand::Reg(Register::R11);
pub const AX: Operand = Operand::Reg(Register::Ax);
pub const DX: Operand = Operand::Reg(Register::Dx);

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
