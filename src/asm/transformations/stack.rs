use crate::asm::{FunctionDefinition, Instruction};
use crate::asm::round_up_16;

pub fn allocate_stack_space(function: &mut FunctionDefinition, bytes_required: u32) {
    let bytes_required = round_up_16(bytes_required);
    function
        .instructions
        .insert(0, Instruction::allocate_stack(bytes_required));
}
