use super::{Program, TopLevel};
use crate::core::Symbols;

mod immediates;
mod operands;
mod pseudoregs;
mod stack;

use immediates::{rewrite_large_imm_values, truncate_movl_imm_value};
use operands::{
    rewrite_invalid_constant_operands, rewrite_invalid_double_memory_instructions,
    rewrite_invalid_imul_memory_dst,
};
use pseudoregs::replace_pseudoregisters;
use stack::allocate_stack_space;

pub fn transform(program: &mut Program, symbols: &Symbols) {
    for toplevel in &mut program.0 {
        if let TopLevel::F(function) = toplevel {
            let bytes_required = replace_pseudoregisters(function, symbols);
            allocate_stack_space(function, bytes_required);
            rewrite_invalid_double_memory_instructions(function);
            rewrite_invalid_imul_memory_dst(function);
            rewrite_invalid_constant_operands(function);
            rewrite_large_imm_values(function);
            truncate_movl_imm_value(function);
        }
    }
}
