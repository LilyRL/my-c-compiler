use crate::asm::{R10, R11};
use crate::asm::{AssemblyType, FunctionDefinition, Instruction, Operand, Register};

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

pub fn rewrite_invalid_constant_operands(function: &mut FunctionDefinition) {
    let mut i = 0;
    while i < function.instructions.len() {
        // TODO: reduce duplication here if you please
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
            Instruction::Div(ty, operand) => {
                if let Operand::Imm(_) = operand {
                    function.instructions[i] = Instruction::Mov {
                        ty,
                        src: operand,
                        dst: R10,
                    };

                    function
                        .instructions
                        .insert(i + 1, Instruction::Div(ty, R10));

                    i += 1;
                }
            }
            Instruction::Movzx { src, dst } => {
                if dst.is_register() {
                    function.instructions[i] = Instruction::Mov {
                        ty: AssemblyType::Longword,
                        src,
                        dst,
                    };
                } else {
                    function.instructions[i] = Instruction::Mov {
                        ty: AssemblyType::Longword,
                        src,
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

                    i += 1;
                }
            }
            _ => (),
        }

        i += 1;
    }
}
