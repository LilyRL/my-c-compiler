mod data;
mod lowering;
use std::collections::BTreeMap;

pub use data::*;

use crate::utils::{align_to, can_fit_in_i32, round_up_16};

pub const R10: Operand = Operand::Reg(Register::R10);
pub const R11: Operand = Operand::Reg(Register::R11);

pub fn transform(program: &mut Program) {
    for toplevel in &mut program.0 {
        if let TopLevel::F(function) = toplevel {
            let bytes_required = replace_pseudoregisters(function);
            allocate_stack_space(function, bytes_required);
            rewrite_invalid_double_memory_instructions(function);
            rewrite_invalid_imul_memory_dst(function);
            rewrite_constant_idiv_operands(function);
            rewrite_large_imm_values(function);
            truncate_movl_imm_value(function);
        }
    }
}

/// returns the number of bytes to allocate for this function
pub fn replace_pseudoregisters(function: &mut FunctionDefinition) -> u32 {
    let mut bytes_allocated = 0;
    let mut map: BTreeMap<String, i32> = BTreeMap::new();

    let mut process_operand = |operand: &mut Operand| {
        if let Operand::Pseudo(ident) = operand {
            if let Some(offset) = map.get(&ident.0) {
                *operand = Operand::Stack(*offset);
            } else if let Some(data) = get_asm_symbols().get(&ident)
                && data.is_static()
            {
                *operand = Operand::Data(ident.clone());
            } else {
                let ty = get_asm_symbols().get(&ident).unwrap().ty().unwrap();
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
            Instruction::Mov { src, dst, .. } => {
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
            Instruction::Idiv(_, operand) => {
                process_operand(operand);
            }
            Instruction::Cmp(_, a, b) => {
                process_operand(a);
                process_operand(b);
            }
            Instruction::SetCC(_, operand) => {
                process_operand(operand);
            }
            Instruction::Push(operand) => {
                process_operand(operand);
            }
            Instruction::Movsx { src, dst } => {
                process_operand(src);
                process_operand(dst);
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

pub fn truncate_movl_imm_value(function: &mut FunctionDefinition) {
    for instruction in function.instructions.iter_mut() {
        if let Instruction::Mov { ty, src, .. } = instruction {
            if *ty == AssemblyType::Longword
                && let Operand::Imm(src) = src
                && !can_fit_in_i32(*src)
            {
                *src = (*src as i32) as i64;
            }
        }
    }
}

pub fn allocate_stack_space(function: &mut FunctionDefinition, bytes_required: u32) {
    let bytes_required = round_up_16(bytes_required);
    function
        .instructions
        .insert(0, Instruction::allocate_stack(bytes_required));
}

pub fn rewrite_large_imm_values(function: &mut FunctionDefinition) {
    let mut i = 0;

    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::Mov { ty, src, dst }
                if ty.is_quadword()
                    && dst.is_memory()
                    && let Operand::Imm(n) = src
                    && !can_fit_in_i32(n) =>
            {
                function.instructions[i] = Instruction::Mov {
                    ty,
                    src: Operand::Reg(Register::R10),
                    dst,
                };
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        ty,
                        src,
                        dst: Operand::Reg(Register::R10),
                    },
                );
                i += 1;
            }
            Instruction::Binary {
                ty,
                operator,
                src,
                dst,
            } if operator.cant_have_large_imm()
                && ty.is_quadword()
                && let Operand::Imm(n) = src
                && !can_fit_in_i32(n) =>
            {
                function.instructions[i] = Instruction::Binary {
                    ty,
                    operator,
                    src: Operand::Reg(Register::R10),
                    dst,
                };
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        ty,
                        src: src,
                        dst: Operand::Reg(Register::R10),
                    },
                );
                i += 1;
            }
            Instruction::Cmp(ty, src, dst)
                if ty.is_quadword()
                    && let Operand::Imm(n) = src
                    && !can_fit_in_i32(n) =>
            {
                function.instructions[i] = Instruction::Cmp(ty, Operand::Reg(Register::R10), dst);
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        ty,
                        src: src,
                        dst: Operand::Reg(Register::R10),
                    },
                );
                i += 1;
            }
            Instruction::Push(op)
                if let Operand::Imm(n) = op
                    && !can_fit_in_i32(n) =>
            {
                function.instructions[i] = Instruction::Push(Operand::Reg(Register::R10));
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        ty: AssemblyType::Quadword,
                        src: op,
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

pub fn rewrite_invalid_double_memory_instructions(function: &mut FunctionDefinition) {
    let mut i = 0;

    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::Mov { src, dst, ty } if src.is_memory() && dst.is_memory() => {
                function.instructions[i] = Instruction::Mov { src: R10, dst, ty };
                function
                    .instructions
                    .insert(i, Instruction::Mov { src, dst: R10, ty });
                i += 1;
            }
            Instruction::Binary {
                operator,
                src,
                dst,
                ty,
            } if operator.cant_have_double_memory() && src.is_memory() && dst.is_memory() => {
                function.instructions[i] = Instruction::Binary {
                    ty,
                    operator,
                    src: R10,
                    dst,
                };
                function
                    .instructions
                    .insert(i, Instruction::Mov { src, dst: R10, ty });
                i += 1;
            }
            Instruction::Binary {
                operator,
                src,
                dst,
                ty,
            } if operator.is_shift() && src.is_memory() => {
                // cnt must be in %ecx
                function.instructions[i] = Instruction::Binary {
                    ty,
                    operator,
                    src: Operand::Reg(Register::Cx),
                    dst,
                };
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        ty,
                        src,
                        dst: Operand::Reg(Register::Cx),
                    },
                );
                i += 1;
            }
            Instruction::Cmp(ty, a, b) if a.is_memory() && b.is_memory() => {
                function.instructions[i] = Instruction::Cmp(ty, R10, b);
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        src: a,
                        dst: R10,
                        ty,
                    },
                );
                i += 1;
            }
            Instruction::Cmp(ty, a, b) if b.is_constant() => {
                function.instructions[i] = Instruction::Cmp(ty, a, R11);
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        src: b,
                        dst: R11,
                        ty,
                    },
                );
                i += 1;
            }
            Instruction::Movsx { src, dst } => {
                let mut inner_src = src.clone();

                if src.is_constant() {
                    inner_src = R10;
                    function.instructions[i] = Instruction::Movsx {
                        src: inner_src.clone(),
                        dst: dst.clone(),
                    };
                    function.instructions.insert(
                        i,
                        Instruction::Mov {
                            src,
                            dst: inner_src.clone(),
                            ty: AssemblyType::Longword,
                        },
                    );
                    i += 1;
                }

                if dst.is_memory() {
                    function.instructions[i] = Instruction::Movsx {
                        src: inner_src,
                        dst: R11,
                    };
                    function.instructions.insert(
                        i + 1,
                        Instruction::Mov {
                            ty: AssemblyType::Quadword,
                            src: R11,
                            dst,
                        },
                    );
                }
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
            Instruction::Binary {
                operator,
                src,
                dst,
                ty,
            } if operator.is_mult() => {
                if dst.is_memory() {
                    function.instructions[i] = Instruction::Mov {
                        ty,
                        src: dst.clone(),
                        dst: R11,
                    };

                    function.instructions.insert(
                        i + 1,
                        Instruction::Binary {
                            ty,
                            operator,
                            src: src,
                            dst: R11,
                        },
                    );

                    function
                        .instructions
                        .insert(i + 2, Instruction::Mov { src: R11, dst, ty });

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
            Instruction::Idiv(ty, operand) => {
                if let Operand::Imm(_) = operand {
                    function.instructions[i] = Instruction::Mov {
                        ty,
                        src: operand,
                        dst: R10,
                    };

                    function
                        .instructions
                        .insert(i + 1, Instruction::Idiv(ty, R10));

                    i += 1;
                }
            }
            _ => (),
        }

        i += 1;
    }
}
