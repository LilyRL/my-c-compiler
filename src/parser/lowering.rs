use super::*;
use crate::ir;
use ir::{Instruction, Value};

impl Program {
    pub fn lower(self) -> ir::Program {
        ir::Program(self.0.lower())
    }
}

impl FunctionDefinition {
    pub fn lower(self) -> ir::FunctionDefinition {
        let mut instructions = Vec::new();

        for statement in self.block {
            statement.lower(&mut instructions);
        }

        ir::FunctionDefinition {
            name: self.name,
            body: instructions,
        }
    }
}

impl BlockItem {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Stmt(stmt) => stmt.lower(instructions),
            Self::Decl(decl) => {
                if let Some(init) = decl.init {
                    let out = init.lower(instructions);
                    let copy = Instruction::Copy {
                        src: out,
                        dst: Value::Var(decl.name),
                    };
                    instructions.push(copy);
                }
            }
        }
    }
}

impl Statement {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Return(c) => {
                let dst = c.lower(instructions);
                instructions.push(Instruction::Return(dst));
            }
            Self::Expression(exp) => {
                exp.lower(instructions);
            }
            Self::If { cond, then, else_ } => {
                let cond = cond.lower(instructions);
                let end_label = Identifier::new("if_end");

                if let Some(else_) = else_ {
                    let else_label = Identifier::new("if_else");
                    instructions.push(Instruction::JumpIfZero {
                        condition: cond,
                        target: else_label.clone(),
                    });
                    then.lower(instructions);
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(else_label));
                    else_.lower(instructions);
                    instructions.push(Instruction::Label(end_label));
                } else {
                    instructions.push(Instruction::JumpIfZero {
                        condition: cond,
                        target: end_label.clone(),
                    });
                    then.lower(instructions);
                    instructions.push(Instruction::Label(end_label));
                }
            }
            Self::Label(i) => instructions.push(Instruction::Label(i)),
            Self::Goto(i) => instructions.push(Instruction::Jump(i)),
            Self::Compound(block) => {
                for item in block {
                    item.lower(instructions);
                }
            }
            Self::Null => (),
        }
    }
}

impl Expression {
    pub fn lower(self, instructions: &mut Vec<Instruction>) -> Value {
        match self {
            Self::Assignment(lhs, rhs) => {
                assert!(
                    lhs.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let v = match *lhs {
                    Self::Var(v) => v,
                    _ => unreachable!(),
                };

                let result = rhs.lower(instructions);
                instructions.push(Instruction::Copy {
                    src: result,
                    dst: Value::Var(v.clone()),
                });
                return Value::Var(v.clone());
            }
            Self::CompoundAssign { operator, lhs, rhs } => {
                assert!(
                    lhs.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let v = match *lhs {
                    Self::Var(v) => v,
                    _ => unreachable!(),
                };

                let lhs_value = Value::Var(v.clone());
                let rhs_value = rhs.lower(instructions);
                let dst = Value::Var(Identifier::new(operator.name()));

                instructions.push(Instruction::Binary {
                    operator: operator.compound_assign().unwrap().lower(),
                    lhs: lhs_value,
                    rhs: rhs_value,
                    dst: dst.clone(),
                });

                instructions.push(Instruction::Copy {
                    src: dst.clone(),
                    dst: Value::Var(v.clone()),
                });

                return dst;
            }
            Self::Var(v) => Value::Var(v),
            Self::Constant(c) => c.lower(),
            Self::Unary { operator, expr } => {
                let src = expr.lower(instructions);
                let dst = Value::Var(Identifier::new("tmp"));

                instructions.push(Instruction::Unary {
                    operator: operator.lower(),
                    src,
                    dst: dst.clone(),
                });
                dst
            }
            Self::Binary { operator, lhs, rhs } if operator.can_be_lowered() => {
                let lhs = lhs.lower(instructions);
                let rhs = rhs.lower(instructions);
                let dst = Value::Var(Identifier::new(operator.name()));

                instructions.push(Instruction::Binary {
                    operator: operator.lower(),
                    lhs,
                    rhs,
                    dst: dst.clone(),
                });
                return dst;
            }
            Self::Binary { operator, lhs, rhs } => match operator {
                BinaryOperator::And => {
                    let dst = Value::Var(Identifier::new("and_result"));
                    let false_label = Identifier::new("and_false");
                    let end_label = Identifier::new("and_end");

                    let lhs = lhs.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: lhs,
                        target: false_label.clone(),
                    });

                    let rhs = rhs.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: rhs,
                        target: false_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(1),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(false_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(0),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                BinaryOperator::Or => {
                    let dst = Value::Var(Identifier::new("or_result"));
                    let true_label = Identifier::new("or_true");
                    let end_label = Identifier::new("or_end");

                    let lhs = lhs.lower(instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: lhs,
                        target: true_label.clone(),
                    });

                    let rhs = rhs.lower(instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: rhs,
                        target: true_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(0),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(true_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(1),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                _ => unreachable!(),
            },
            Self::Prefix(op, expr) => {
                assert!(
                    expr.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let expr = expr.lower(instructions);

                instructions.push(Instruction::Binary {
                    operator: ir::BinaryOperator::Add,
                    lhs: expr.clone(),
                    rhs: Value::Constant(op.n()),
                    dst: expr.clone(),
                });

                expr
            }
            Self::Postfix(op, expr) => {
                assert!(
                    expr.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let lhs = expr.lower(instructions);
                let old_value = Value::Var(Identifier::new("postfix_old_value"));

                instructions.push(Instruction::Copy {
                    src: lhs.clone(),
                    dst: old_value.clone(),
                });

                instructions.push(Instruction::Binary {
                    operator: ir::BinaryOperator::Add,
                    lhs: lhs.clone(),
                    rhs: Value::Constant(op.n()),
                    dst: lhs.clone(),
                });

                old_value
            }
            Self::Conditional(cond, if_true, if_false) => {
                let cond = cond.lower(instructions);
                let else_label = Identifier::new("conditional_else");
                let end_label = Identifier::new("conditional_end");
                let result = Value::Var(Identifier::new("conditional_result"));

                instructions.push(Instruction::JumpIfZero {
                    condition: cond,
                    target: else_label.clone(),
                });

                let if_true = if_true.lower(instructions);
                instructions.push(Instruction::Copy {
                    src: if_true,
                    dst: result.clone(),
                });
                instructions.push(Instruction::Jump(end_label.clone()));

                instructions.push(Instruction::Label(else_label));
                let if_false = if_false.lower(instructions);
                instructions.push(Instruction::Copy {
                    src: if_false,
                    dst: result.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                result
            }
        }
    }
}
