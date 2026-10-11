use crate::asm::{FunctionDefinition, Instruction, Operand};

pub fn rewrite_mov_0(function: &mut FunctionDefinition) {
    for instruction in &mut function.instructions {
        if let Instruction::Mov { ty, src, dst } = instruction
            && let Operand::Imm(0i64) = src
        {
            *instruction = Instruction::ZeroOut(*ty, dst.clone());
        }
    }
}
