use crate::asm::round_up_16;
use crate::asm::{FunctionDefinition, Instruction};

pub fn allocate_stack_space(function: &mut FunctionDefinition, bytes_required: u32) {
    // 8 more bytes for the return address that call pushes
    let bytes_required = round_up_16(bytes_required + 8);
    function
        .instructions
        .insert(0, Instruction::allocate_stack(bytes_required));
}
