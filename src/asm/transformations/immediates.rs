use crate::asm::can_fit_in_i32;
use crate::asm::{AssemblyType, FunctionDefinition, Instruction, Operand, Register};

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
