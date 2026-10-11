use crate::asm::{AssemblyType, FunctionDefinition, Instruction, Operand, Register, types};
use crate::asm::{R10, R11, XMM10, XMM11};

fn scratch(ty: AssemblyType) -> Operand {
    if ty.is_float() { XMM11 } else { R10 }
}

fn second_scratch(ty: AssemblyType) -> Operand {
    if ty.is_float() { XMM10 } else { R11 }
}

pub fn rewrite_invalid_double_memory_instructions(function: &mut FunctionDefinition) {
    let mut i = 0;

    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::SetCC(c, op) if op.is_memory() => {
                let reg = R11;
                function.instructions[i] = Instruction::SetCC(c, reg.clone());

                // we have to zero out the register, but can't use xor because that clobbers flags
                // stupid
                function.instructions.insert(
                    i + 1,
                    Instruction::Binary {
                        ty: AssemblyType::Longword,
                        operator: crate::asm::BinaryOperator::BitwiseAnd,
                        src: Operand::Imm(0xff),
                        dst: reg.clone(),
                    },
                );
                function.instructions.insert(
                    i + 2,
                    Instruction::Mov {
                        src: reg,
                        dst: op,
                        ty: AssemblyType::Longword,
                    },
                );
                i += 2;
            }
            Instruction::Mov { src, dst, ty } if src.is_memory() && dst.is_memory() => {
                let reg = scratch(ty);
                function.instructions[i] = Instruction::Mov {
                    src: reg.clone(),
                    dst,
                    ty,
                };
                function
                    .instructions
                    .insert(i, Instruction::Mov { src, dst: reg, ty });
                i += 1;
            }
            Instruction::ZeroOut(ty, op) if op.is_memory() => {
                let reg = scratch(ty);
                function.instructions[i] = Instruction::ZeroOut(ty, reg.clone());
                function.instructions.insert(
                    i + 1,
                    Instruction::Mov {
                        src: reg,
                        dst: op,
                        ty,
                    },
                );
                i += 1;
            }
            Instruction::Binary {
                operator: crate::asm::BinaryOperator::BitwiseXor,
                src,
                dst,
                ty,
            } if src.is_memory() || dst.is_memory() => {
                let reg = scratch(ty);
                let reg2 = second_scratch(ty);
                function.instructions[i] = Instruction::Mov {
                    ty,
                    src: reg.clone(),
                    dst: reg2.clone(),
                };
                function.instructions.insert(
                    i,
                    Instruction::Binary {
                        ty,
                        operator: crate::asm::BinaryOperator::BitwiseXor,
                        src,
                        dst: reg.clone(),
                    },
                );
                function
                    .instructions
                    .insert(i + 2, Instruction::Mov { ty, src: reg2, dst });
                i += 2;
            }
            Instruction::Binary {
                operator,
                src,
                dst,
                ty,
            } if operator.cant_have_double_memory() && src.is_memory() && dst.is_memory() => {
                let reg = scratch(ty);
                function.instructions[i] = Instruction::Binary {
                    ty,
                    operator,
                    src,
                    dst: reg.clone(),
                };
                function
                    .instructions
                    .insert(i + 1, Instruction::Mov { src: reg, dst, ty });
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
                let reg = scratch(ty);
                function.instructions[i] = Instruction::Cmp(ty, reg.clone(), b);
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        src: a,
                        dst: reg,
                        ty,
                    },
                );
                i += 1;
            }
            Instruction::Cmp(ty, a, b) if b.is_constant() && !ty.is_float() => {
                // floats are never immidiate
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
            Instruction::Convert {
                src,
                src_type,
                dst,
                dst_type,
            } if dst.is_memory() => {
                let reg = scratch(dst_type);
                function.instructions[i] = Instruction::Convert {
                    src: src,
                    src_type,
                    dst: reg.clone(),
                    dst_type,
                };
                function.instructions.insert(
                    i + 1,
                    Instruction::Mov {
                        src: reg,
                        dst,
                        ty: dst_type,
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

pub fn rewrite_invalid_float_memory_dst(function: &mut FunctionDefinition) {
    let mut i = 0;

    while i < function.instructions.len() {
        match function.instructions[i].clone() {
            Instruction::Binary {
                ty,
                operator,
                src,
                dst,
            } if ty.is_float() && dst.is_memory() => {
                if operator.is_bitwise_xor() {
                    let reg = scratch(ty);
                    function.instructions[i] = Instruction::Binary {
                        ty,
                        operator: crate::asm::BinaryOperator::BitwiseXor,
                        src: src.clone(),
                        dst: reg.clone(),
                    };
                    function.instructions.insert(
                        i + 1,
                        Instruction::Mov {
                            src: reg,
                            dst: dst.clone(),
                            ty,
                        },
                    );
                    i += 1;
                }

                let reg = second_scratch(ty);

                function.instructions[i] = Instruction::Mov {
                    ty,
                    src: dst.clone(),
                    dst: reg.clone(),
                };
                function.instructions.insert(
                    i + 1,
                    Instruction::Binary {
                        ty,
                        operator,
                        src,
                        dst: reg.clone(),
                    },
                );
                function
                    .instructions
                    .insert(i + 2, Instruction::Mov { src: reg, dst, ty });

                i += 2;
            }
            Instruction::Cmp(ty, a, b) if ty.is_float() && b.is_memory() => {
                // the both memory rule above already staged a into the first
                // scratch register, so it must not be clobbered here
                let reg = match a == scratch(ty) {
                    true => second_scratch(ty),
                    false => scratch(ty),
                };
                function.instructions[i] = Instruction::Cmp(ty, a, reg.clone());
                function.instructions.insert(
                    i,
                    Instruction::Mov {
                        src: b,
                        dst: reg,
                        ty,
                    },
                );
                i += 1;
            }
            Instruction::TruncateFloat { ty, src, dst } if src.is_memory() || dst.is_memory() => {
                let src_reg = scratch(ty);
                let dst_reg = second_scratch(ty);
                let mut rewritten = Vec::new();

                if src.is_memory() {
                    rewritten.push(Instruction::Mov {
                        ty,
                        src: src.clone(),
                        dst: src_reg.clone(),
                    });
                }

                if dst.is_memory() {
                    rewritten.push(Instruction::Mov {
                        ty,
                        src: dst.clone(),
                        dst: dst_reg.clone(),
                    });
                }

                rewritten.push(Instruction::TruncateFloat {
                    ty,
                    src: match src.is_memory() {
                        true => src_reg,
                        false => src,
                    },
                    dst: match dst.is_memory() {
                        true => dst_reg.clone(),
                        false => dst.clone(),
                    },
                });

                if dst.is_memory() {
                    rewritten.push(Instruction::Mov {
                        src: dst_reg,
                        dst,
                        ty,
                    });
                }

                let extra = rewritten.len() - 1;
                function.instructions.splice(i..=i, rewritten);
                i += extra;
            }
            Instruction::ZeroOut(ty, op) if ty.is_float() && op.is_memory() => {
                let reg = scratch(ty);
                function.instructions[i] = Instruction::ZeroOut(ty, reg.clone());
                function.instructions.insert(
                    i + 1,
                    Instruction::Mov {
                        src: reg,
                        dst: op,
                        ty,
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
            Instruction::Binary {
                operator,
                src,
                dst,
                ty,
            } if operator.is_mult() && !ty.is_float() => {
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
            Instruction::Cmp(ty, a, b) if ty.is_float() && a.is_constant() => {
                function.instructions[i] = Instruction::Mov {
                    ty,
                    src: a,
                    dst: scratch(ty),
                };

                function
                    .instructions
                    .insert(i + 1, Instruction::Cmp(ty, scratch(ty), b));

                i += 1;
            }
            Instruction::Convert {
                src,
                src_type,
                dst,
                dst_type,
            } => {
                // uhh dont think this can even happen but you never know...
                if src.is_constant() {
                    // the load has to come first, so it goes in before the
                    // convert it feeds
                    function.instructions.insert(
                        i,
                        Instruction::Mov {
                            ty: src_type,
                            src,
                            dst: R10,
                        },
                    );
                    function.instructions[i + 1] = Instruction::Convert {
                        src: R10,
                        src_type,
                        dst,
                        dst_type,
                    };
                    i += 1;
                }
            }
            _ => (),
        }

        i += 1;
    }
}
