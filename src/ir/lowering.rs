use std::collections::HashMap;

use super::*;

use crate::{
    analysis::{Type, get_symbols},
    codegen::{
        self, AsmSymbol, AsmSymbols, AssemblyType, CondCode, Operand, Register,
        set_asm_symbol_table,
    },
};

const ARG_REGISTERS: [Register; 6] = {
    use Register::*;
    [Di, Si, Dx, Cx, R8, R9]
};

impl Value {
    pub fn lower(self) -> codegen::Operand {
        match self {
            Self::Constant(int) => codegen::Operand::Imm(int.i64()),
            Self::Var(ident) => codegen::Operand::Pseudo(ident),
        }
    }
}

impl Instruction {
    pub fn lower(self, instructions: &mut Vec<codegen::Instruction>) {
        match self {
            Self::Return(val) => {
                instructions.push(codegen::Instruction::Mov {
                    ty: val.asm_type(),
                    src: val.lower(),
                    dst: codegen::Operand::Reg(Register::Ax),
                });
                instructions.push(codegen::Instruction::Ret);
            }
            Self::Unary {
                operator: UnaryOperator::Not,
                src,
                dst,
            } => {
                let ty = src.asm_type();
                let src = src.lower();
                let dst_ty = dst.asm_type();
                let dst = dst.lower();

                instructions.push(codegen::Instruction::Cmp(ty, Operand::Imm(0), src));
                instructions.push(codegen::Instruction::Mov {
                    ty: dst_ty,
                    src: Operand::Imm(0),
                    dst: dst.clone(),
                });
                instructions.push(codegen::Instruction::SetCC(CondCode::Eq, dst));
            }
            Self::Unary { operator, src, dst } => {
                let ty = dst.asm_type();
                let dst = dst.lower();
                instructions.push(codegen::Instruction::Mov {
                    ty,
                    src: src.lower(),
                    dst: dst.clone(),
                });
                instructions.push(codegen::Instruction::Unary {
                    ty,
                    operator: operator.lower(),
                    operand: dst,
                });
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let ty = lhs.asm_type();
                let dst_ty = dst.asm_type();
                let dst = dst.lower();
                let lhs = lhs.lower();
                let rhs = rhs.lower();

                match operator {
                    BinaryOperator::Divide | BinaryOperator::Remainder => {
                        let register = match operator {
                            BinaryOperator::Divide => Register::Ax,
                            _ => Register::Dx,
                        };

                        instructions.push(codegen::Instruction::Mov {
                            ty,
                            src: lhs,
                            dst: Operand::Reg(Register::Ax),
                        });
                        instructions.push(codegen::Instruction::Cdq(ty));
                        instructions.push(codegen::Instruction::Idiv(ty, rhs));
                        instructions.push(codegen::Instruction::Mov {
                            ty,
                            src: Operand::Reg(register),
                            dst,
                        });
                    }
                    BinaryOperator::GreaterThan
                    | BinaryOperator::LessThan
                    | BinaryOperator::GreaterEqual
                    | BinaryOperator::LessEqual
                    | BinaryOperator::Equal
                    | BinaryOperator::NotEqual => {
                        let cond_code = match operator {
                            BinaryOperator::GreaterThan => CondCode::Gt,
                            BinaryOperator::LessThan => CondCode::Lt,
                            BinaryOperator::GreaterEqual => CondCode::Ge,
                            BinaryOperator::LessEqual => CondCode::Le,
                            BinaryOperator::Equal => CondCode::Eq,
                            _ => CondCode::Ne,
                        };

                        instructions.push(codegen::Instruction::Cmp(ty, rhs, lhs));
                        instructions.push(codegen::Instruction::Mov {
                            ty: dst_ty,
                            src: Operand::Imm(0),
                            dst: dst.clone(),
                        });
                        instructions.push(codegen::Instruction::SetCC(cond_code, dst));
                    }

                    _ => {
                        instructions.push(codegen::Instruction::Mov {
                            ty,
                            src: lhs,
                            dst: dst.clone(),
                        });
                        instructions.push(codegen::Instruction::Binary {
                            ty,
                            operator: operator.lower(),
                            src: rhs,
                            dst,
                        });
                    }
                }
            }
            Self::Jump(label) => instructions.push(codegen::Instruction::Jump(label)),
            Self::JumpIfZero { condition, target } => {
                let ty = condition.asm_type();
                let condition = condition.lower();
                instructions.push(codegen::Instruction::Cmp(ty, Operand::Imm(0), condition));
                instructions.push(codegen::Instruction::JumpCC(CondCode::Eq, target))
            }
            Self::JumpNotZero { condition, target } => {
                let ty = condition.asm_type();
                let cond = condition.lower();
                instructions.push(codegen::Instruction::Cmp(ty, Operand::Imm(0), cond));
                instructions.push(codegen::Instruction::JumpCC(CondCode::Ne, target))
            }
            Self::Label(label) => instructions.push(codegen::Instruction::Label(label)),
            Self::Copy { src, dst } => instructions.push(codegen::Instruction::Mov {
                ty: src.asm_type(),
                src: src.lower(),
                dst: dst.lower(),
            }),
            Self::Comment(c) => instructions.push(codegen::Instruction::Comment(c)),
            Self::FunctionCall { name, args, dst } => {
                let ty = dst.asm_type();
                let mut register_args = Vec::with_capacity(args.len().min(6));
                let mut stack_args = Vec::with_capacity(args.len().saturating_sub(6));
                for (i, arg) in args.into_iter().enumerate() {
                    if i < 6 {
                        register_args.push(arg);
                    } else {
                        stack_args.push(arg);
                    }
                }

                // TODO: this assumes that all values are 32 bits, fine for now but always be on the lookout
                let stack_padding = if stack_args.len().is_multiple_of(2) {
                    0
                } else {
                    8
                };

                if stack_padding != 0 {
                    instructions.push(codegen::Instruction::allocate_stack(stack_padding));
                }

                let bytes_to_remove = 8 * stack_args.len() as u32 + stack_padding;

                for (i, tacky_arg) in register_args.into_iter().enumerate() {
                    let r = ARG_REGISTERS[i];
                    let arg_ty = tacky_arg.asm_type();
                    let assembly_arg = tacky_arg.lower();
                    instructions.push(codegen::Instruction::Mov {
                        ty: arg_ty,
                        src: assembly_arg,
                        dst: Operand::Reg(r),
                    });
                }

                for tacky_arg in stack_args.into_iter().rev() {
                    let asm_type = tacky_arg.asm_type();
                    let assembly_arg = tacky_arg.lower();
                    if matches!(assembly_arg, Operand::Reg(_) | Operand::Imm(_))
                        || asm_type == AssemblyType::Quadword
                    {
                        instructions.push(codegen::Instruction::Push(assembly_arg));
                    } else {
                        // stack operands can't be pushed directly
                        instructions.push(codegen::Instruction::Mov {
                            ty: AssemblyType::Longword,
                            src: assembly_arg,
                            dst: Operand::Reg(Register::Ax),
                        });
                        instructions.push(codegen::Instruction::Push(Operand::Reg(Register::Ax)));
                    }
                }

                instructions.push(codegen::Instruction::Call(name));

                if bytes_to_remove != 0 {
                    instructions.push(codegen::Instruction::deallocate_stack(bytes_to_remove));
                }

                let dst = dst.lower();
                instructions.push(codegen::Instruction::Mov {
                    ty,
                    src: Operand::Reg(Register::Ax),
                    dst,
                });
            }
            Self::SignExtend { src, dst } => {
                let src = src.lower();
                let dst = dst.lower();
                instructions.push(codegen::Instruction::Movsx { src, dst });
            }
            Self::Truncate { src, dst } => {
                let src = src.lower();
                let dst = dst.lower();
                instructions.push(codegen::Instruction::Mov {
                    ty: AssemblyType::Longword,
                    src,
                    dst,
                });
            }
        }
    }
}

impl FunctionDefinition {
    pub fn lower(self) -> codegen::FunctionDefinition {
        let mut instructions = vec![];
        let FunctionDefinition {
            name,
            params,
            body,
            global,
        } = self;

        for (i, p) in params.into_iter().enumerate() {
            if i < 6 {
                let r = ARG_REGISTERS[i];
                instructions.push(codegen::Instruction::Mov {
                    ty: p.ty.to_asm_type().unwrap(),
                    src: codegen::Operand::Reg(r),
                    dst: codegen::Operand::Pseudo(p.name),
                });
            } else {
                let offset = 8 * (i as u32 - 6) + 16;
                instructions.push(codegen::Instruction::Mov {
                    ty: p.ty.to_asm_type().unwrap(),
                    src: codegen::Operand::Stack(offset as i32),
                    dst: codegen::Operand::Pseudo(p.name),
                });
            }
        }

        for instruction in body {
            instruction.lower(&mut instructions);
        }

        codegen::FunctionDefinition {
            name: name,
            instructions,
            global,
        }
    }
}

impl StaticVariable {
    pub fn lower(self) -> codegen::StaticVariable {
        codegen::StaticVariable {
            alignment: self.ty.alignment(),
            name: self.name,
            global: self.global,
            init: self.init,
        }
    }
}

impl Program {
    pub fn lower(self) -> codegen::Program {
        let program = codegen::Program(self.0.into_iter().map(|f| f.lower()).collect());

        let frontend_symbols = get_symbols();
        let mut backend_symbols: AsmSymbols = HashMap::new();

        for (i, symbol) in frontend_symbols.iter() {
            let symbol = match &symbol.ty {
                Type::Int => AsmSymbol::Object {
                    ty: AssemblyType::Longword,
                    is_static: symbol.attributes.is_static(),
                },
                Type::Long => AsmSymbol::Object {
                    ty: AssemblyType::Quadword,
                    is_static: symbol.attributes.is_static(),
                },
                Type::Function(f) => AsmSymbol::Function { defined: f.defined },
            };

            backend_symbols.insert(i.clone(), symbol);
        }

        set_asm_symbol_table(backend_symbols);

        program
    }
}

impl TopLevel {
    pub fn lower(self) -> codegen::TopLevel {
        match self {
            Self::F(f) => codegen::TopLevel::F(f.lower()),
            Self::V(v) => codegen::TopLevel::V(v.lower()),
        }
    }
}

impl UnaryOperator {
    pub fn lower(self) -> codegen::UnaryOperator {
        match self {
            Self::Negate => codegen::UnaryOperator::Negate,
            Self::BitwiseNot => codegen::UnaryOperator::BitwiseNot,
            Self::Not => codegen::UnaryOperator::Not,
        }
    }
}

impl BinaryOperator {
    pub fn lower(self) -> codegen::BinaryOperator {
        match self {
            Self::Add => codegen::BinaryOperator::Add,
            Self::Subtract => codegen::BinaryOperator::Sub,
            Self::Multiply => codegen::BinaryOperator::Mul,
            Self::LeftShift => codegen::BinaryOperator::LeftShift,
            Self::RightShift => codegen::BinaryOperator::RightShift,
            Self::BitwiseAnd => codegen::BinaryOperator::BitwiseAnd,
            Self::BitwiseXor => codegen::BinaryOperator::BitwiseXor,
            Self::BitwiseOr => codegen::BinaryOperator::BitwiseOr,
            Self::NotEqual => codegen::BinaryOperator::NotEqual,
            Self::LessThan => codegen::BinaryOperator::LessThan,
            Self::LessEqual => codegen::BinaryOperator::LessEqual,
            Self::GreaterThan => codegen::BinaryOperator::GreaterThan,
            Self::GreaterEqual => codegen::BinaryOperator::GreaterEqual,
            _ => unimplemented!(),
        }
    }
}
