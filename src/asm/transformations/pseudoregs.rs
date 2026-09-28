use std::collections::BTreeMap;

use crate::asm::align_to;
use crate::{
    asm::{FunctionDefinition, Instruction, Operand},
    core::Symbols,
    target::target_os,
};

pub fn replace_pseudoregisters(function: &mut FunctionDefinition, symbols: &Symbols) -> u32 {
    let mut bytes_allocated = 0;
    let mut map: BTreeMap<String, i32> = BTreeMap::new();

    let mut process_operand = |operand: &mut Operand| {
        if let Operand::Pseudo(ident) = operand {
            if let Some(offset) = map.get(&ident.0) {
                *operand = Operand::Stack(*offset);
            } else if let Some(data) = symbols.get(ident)
                && data.attributes.is_static()
            {
                let global = data.attributes.global();
                *operand = Operand::Data(target_os().decorate_symbol(&ident.0, global));
            } else {
                let ty = symbols
                    .get(ident)
                    .expect("every pseudo-register must have a symbol")
                    .asm_type()
                    .expect("function names are never used as operands");
                let size = ty.size_bytes();
                let alignment = ty.alignment();
                bytes_allocated = align_to(bytes_allocated, alignment);
                bytes_allocated += size as i32;
                map.insert(ident.0.clone(), -bytes_allocated);
                *operand = Operand::Stack(-bytes_allocated);
            }
        }
    };

    for instruction in function.instructions.iter_mut() {
        match instruction {
            Instruction::Mov { src, dst, .. }
            | Instruction::Movsx { src, dst }
            | Instruction::Movzx { src, dst }
            | Instruction::Binary { src, dst, .. }
            | Instruction::Cmp(_, src, dst) => {
                process_operand(src);
                process_operand(dst);
            }
            Instruction::SetCC(_, operand)
            | Instruction::Unary { operand, .. }
            | Instruction::Idiv(_, operand)
            | Instruction::Div(_, operand)
            | Instruction::ZeroOut(_, operand)
            | Instruction::Push(operand) => {
                process_operand(operand);
            }
            Instruction::Jump(_)
            | Instruction::JumpCC(_, _)
            | Instruction::Label(_)
            | Instruction::Call(_)
            | Instruction::Cdq(_)
            | Instruction::Comment(_)
            | Instruction::Ret => {}
        }
    }

    bytes_allocated as u32
}
