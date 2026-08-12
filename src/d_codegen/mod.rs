mod data;
use std::collections::BTreeMap;

pub use data::*;

pub fn transform(program: &mut Program) {
    let bytes_required = replace_pseudoregisters(&mut program.0);
    allocate_stack_space(&mut program.0, bytes_required);
    rewrite_invalid_double_memory_instructions(&mut program.0);
    rewrite_invalid_imul_memory_dst(&mut program.0);
    rewrite_constant_idiv_operands(&mut program.0);
}

/// returns the number of bytes to allocate for this function
pub fn replace_pseudoregisters(function: &mut FunctionDefinition) -> u32 {
    let mut bytes_allocated = 0;
    let mut map: BTreeMap<String, i32> = BTreeMap::new();

    let mut process_operand = |operand: &mut Operand| {
        if let Operand::Pseudo(ident) = operand {
            if let Some(offset) = map.get(&ident.0) {
                *operand = Operand::Stack(*offset);
            } else {
                bytes_allocated += 4;
                map.insert(ident.0.clone(), -bytes_allocated);
                *operand = Operand::Stack(-bytes_allocated);
            }
        }
    };

    for instruction in function.instructions.iter_mut() {
        match instruction {
            Instruction::Mov { src, dst } => {
                process_operand(src);
                process_operand(dst);
            }
            Instruction::Unary { operand, .. } => {
                process_operand(operand);
            }
            Instruction::Binary { src, dst, .. } => {
                process_operand(src);
                process_operand(dst);
            }
            Instruction::Idiv(operand) => {
                process_operand(operand);
            }
            Instruction::Cdq | Instruction::AllocateStack(_) | Instruction::Ret => {}
        }
    }

    bytes_allocated as u32
}

pub fn allocate_stack_space(function: &mut FunctionDefinition, bytes_required: u32) {
    let bytes_required = ((bytes_required / 16) + 1) * 16;
    function
        .instructions
        .insert(0, Instruction::AllocateStack(bytes_required));
}

pub fn rewrite_invalid_double_memory_instructions(function: &mut FunctionDefinition) {
    let mut i = 0;

    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::Mov { src, dst } if src.is_stack() && dst.is_stack() => {
                function.instructions[i] = Instruction::Mov {
                    src: Operand::Reg(Register::R10),
                    dst,
                };
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        src,
                        dst: Operand::Reg(Register::R10),
                    },
                );
                i += 1;
            }
            Instruction::Binary { operator, src, dst }
                if operator.is_add_or_sub() && src.is_stack() && dst.is_stack() =>
            {
                function.instructions[i] = Instruction::Binary {
                    operator,
                    src: Operand::Reg(Register::R10),
                    dst,
                };
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        src,
                        dst: Operand::Reg(Register::R10),
                    },
                );
                i += 1;
            }
            _ => (),
        }

        i += 1;
    }
}

pub fn rewrite_invalid_imul_memory_dst(function: &mut FunctionDefinition) {
    let mut i = 0;

    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::Binary { operator, src, dst }
                if matches!(operator, BinaryOperator::Mult) =>
            {
                if dst.is_stack() {
                    function.instructions[i] = Instruction::Mov {
                        src: dst.clone(),
                        dst: Operand::Reg(Register::R11),
                    };

                    function.instructions.insert(
                        i + 1,
                        Instruction::Binary {
                            operator,
                            src: src,
                            dst: Operand::Reg(Register::R11),
                        },
                    );

                    function.instructions.insert(
                        i + 2,
                        Instruction::Mov {
                            src: Operand::Reg(Register::R11),
                            dst: dst,
                        },
                    );

                    i += 2;
                }
            }
            _ => (),
        }

        i += 1;
    }
}

pub fn rewrite_constant_idiv_operands(function: &mut FunctionDefinition) {
    let mut i = 0;
    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::Idiv(operand) => {
                if let Operand::Imm(_) = operand {
                    function.instructions[i] = Instruction::Mov {
                        src: operand,
                        dst: Operand::Reg(Register::R10),
                    };

                    function
                        .instructions
                        .insert(i + 1, Instruction::Idiv(Operand::Reg(Register::R10)));

                    i += 1;
                }
            }
            _ => (),
        }

        i += 1;
    }
}
