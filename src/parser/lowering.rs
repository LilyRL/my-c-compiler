use super::*;
use crate::{
    analysis::{IdentifierAttributes, InitialValue, StaticInit, Symbol, get_symbols},
    ir::{self, StaticVariable, TopLevel},
};
use ir::{Instruction, Value};

impl Program {
    pub fn lower(self) -> ir::Program {
        let mut definitions = self
            .0
            .into_iter()
            .flat_map(|d| match d {
                Declaration::Func(f) => f.lower().map(|f| TopLevel::F(f)),
                _ => None,
            })
            .collect::<Vec<_>>();

        for (name, entry) in get_symbols().iter() {
            match &entry.attributes {
                IdentifierAttributes::Static { init, global } => match init {
                    InitialValue::Constant(i) => definitions.push(TopLevel::V(StaticVariable {
                        name: name.clone(),
                        global: *global,
                        init: *i,
                        ty: entry.ty.clone(),
                    })),
                    InitialValue::Tentitive => definitions.push(TopLevel::V(StaticVariable {
                        name: name.clone(),
                        global: *global,
                        init: match entry.ty {
                            Type::Int => StaticInit::Int(0),
                            Type::Long => StaticInit::Long(0),
                            _ => unreachable!(),
                        },
                        ty: entry.ty.clone(),
                    })),
                    InitialValue::None => (),
                },
                _ => (),
            }
        }

        ir::Program(definitions)
    }
}

impl FunctionDeclaration {
    pub fn lower(self) -> Option<ir::FunctionDefinition> {
        let mut instructions = Vec::new();

        let global = get_symbols()
            .get(&self.name)
            .map(|s| s.attributes.global())
            .unwrap_or_else(|| !self.storage_class.is_static());

        for statement in self.body? {
            statement.lower(&mut instructions);
        }

        Some(ir::FunctionDefinition {
            params: self.params,
            name: self.name,
            body: instructions,
            global,
        })
    }
}

impl BlockItem {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Stmt(stmt) => stmt.lower(instructions),
            Self::Decl(decl) => decl.lower(instructions),
        }
    }
}

impl Declaration {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Func(_) => {}
            Self::Var(var) => var.lower(instructions),
        }
    }
}

impl Statement {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self.kind {
            StmtKind::Return(c) => {
                let dst = c.lower(instructions);
                instructions.push(Instruction::Return(dst));
            }
            StmtKind::Expression(exp) => {
                exp.lower(instructions);
            }
            StmtKind::If { cond, then, else_ } => {
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
            StmtKind::DoWhile { body, cond, label } => {
                let start_label = label._start();
                let continue_label = label._continue();
                let break_label = label._break();

                instructions.push(Instruction::Label(start_label.clone()));
                body.lower(instructions);
                instructions.push(Instruction::Label(continue_label));
                let result = cond.lower(instructions);
                instructions.push(Instruction::JumpNotZero {
                    condition: result,
                    target: start_label,
                });
                instructions.push(Instruction::Label(break_label));
            }
            StmtKind::While { cond, body, label } => {
                let continue_label = label._continue();
                let break_label = label._break();

                instructions.push(Instruction::Label(continue_label.clone()));
                let result = cond.lower(instructions);
                instructions.push(Instruction::JumpIfZero {
                    condition: result,
                    target: break_label.clone(),
                });
                body.lower(instructions);
                instructions.push(Instruction::Jump(continue_label));
                instructions.push(Instruction::Label(break_label));
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
                label,
            } => {
                let start_label = label._start();
                let continue_label = label._continue();
                let break_label = label._break();

                instructions.push(Instruction::Comment("for loop"));
                instructions.push(Instruction::Comment("init"));
                init.lower(instructions);
                instructions.push(Instruction::Label(start_label.clone()));

                if let Some(condition) = condition {
                    instructions.push(Instruction::Comment("for loop condition"));
                    let cond_result = condition.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: cond_result,
                        target: break_label.clone(),
                    });
                } else {
                    instructions.push(Instruction::Comment("no for loop condition"));
                }

                instructions.push(Instruction::Comment("for loop body"));
                body.lower(instructions);
                instructions.push(Instruction::Label(continue_label));
                if let Some(post) = post {
                    post.lower(instructions);
                }
                instructions.push(Instruction::Jump(start_label));
                instructions.push(Instruction::Label(break_label));
            }
            StmtKind::Label(i, stmt) => {
                instructions.push(Instruction::Label(i));
                stmt.lower(instructions);
            }
            StmtKind::Goto(i) => instructions.push(Instruction::Jump(i)),
            StmtKind::Compound(block) => {
                for item in block {
                    item.lower(instructions);
                }
            }
            StmtKind::Break(i) => {
                instructions.push(Instruction::Jump(i._break()));
            }
            StmtKind::Continue(i) => {
                instructions.push(Instruction::Jump(i._continue()));
            }
            StmtKind::Case { label, stmt, .. } | StmtKind::DefaultCase { label, stmt, .. } => {
                instructions.push(Instruction::Label(label));
                stmt.lower(instructions);
            }
            StmtKind::Switch(Switch {
                value,
                body,
                label,
                cases,
                default_case,
                ..
            }) => {
                let result = value.lower(instructions);
                let is_equal_name = Identifier::new("switch_case_is_equal");
                let is_equal = Value::Var(is_equal_name.clone());
                get_symbols().insert(
                    is_equal_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: Type::Int,
                    },
                );
                let break_label = label._break();

                for (label, constant) in cases {
                    let case_value = Value::Constant(constant);
                    instructions.push(Instruction::Binary {
                        operator: ir::BinaryOperator::Equal,
                        lhs: case_value,
                        rhs: result.clone(),
                        dst: is_equal.clone(),
                    });
                    instructions.push(Instruction::JumpNotZero {
                        condition: is_equal.clone(),
                        target: label,
                    });
                }

                if let Some(default) = default_case {
                    instructions.push(Instruction::Jump(default));
                } else {
                    instructions.push(Instruction::Jump(break_label.clone()));
                }

                body.lower(instructions);

                instructions.push(Instruction::Label(break_label));
            }
            StmtKind::Null => (),
        }
    }
}

impl VariableDeclaration {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        if let Some(init) = self.init
            && self.storage_class.is_none()
        {
            let out = init.lower(instructions);
            let copy = Instruction::Copy {
                src: out,
                dst: Value::Var(self.name),
            };
            instructions.push(copy);
        }
    }
}

impl ForInit {
    pub fn lower(self, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Decl(decl) => {
                decl.lower(instructions);
            }
            Self::Expr(expr) => {
                expr.lower(instructions);
            }
            Self::None => (),
        }
    }
}

impl Expression {
    pub fn lower(self, instructions: &mut Vec<Instruction>) -> Value {
        match self.kind {
            ExprKind::Assignment(lhs, rhs) => {
                assert!(
                    lhs.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let v = lhs.as_var().cloned().unwrap();

                let result = rhs.lower(instructions);
                instructions.push(Instruction::Copy {
                    src: result,
                    dst: Value::Var(v.clone()),
                });
                return Value::Var(v.clone());
            }
            ExprKind::CompoundAssign { .. } => {
                unreachable!()
            }
            ExprKind::Var(v) => Value::Var(v),
            ExprKind::Constant(c) => c.lower(),
            ExprKind::Unary { operator, expr } => {
                let dst_name = Identifier::new("tmp");
                let dst = Value::Var(dst_name.clone());
                get_symbols().insert(
                    dst_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: expr.ty.clone(),
                    },
                );

                let src = expr.lower(instructions);

                instructions.push(Instruction::Unary {
                    operator: operator.lower(),
                    src,
                    dst: dst.clone(),
                });
                dst
            }
            ExprKind::Binary { operator, lhs, rhs } if operator.can_be_lowered() => {
                let dst_name = Identifier::new(operator.name());
                let dst = Value::Var(dst_name.clone());
                get_symbols().insert(
                    dst_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: lhs.ty.clone(),
                    },
                );
                let lhs = lhs.lower(instructions);
                let rhs = rhs.lower(instructions);

                instructions.push(Instruction::Binary {
                    operator: operator.lower(),
                    lhs,
                    rhs,
                    dst: dst.clone(),
                });
                return dst;
            }
            ExprKind::Binary { operator, lhs, rhs } => match operator {
                BinaryOperator::And => {
                    let dst_name = Identifier::new("and_result");
                    let dst = Value::Var(dst_name.clone());
                    get_symbols().insert(
                        dst_name,
                        Symbol {
                            attributes: IdentifierAttributes::Local,
                            ty: Type::Int,
                        },
                    );
                    let false_label = Identifier::new("and_false");
                    let end_label = Identifier::new("and_end");

                    let lhs = lhs.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: lhs.clone(),
                        target: false_label.clone(),
                    });

                    let rhs = rhs.lower(instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: rhs,
                        target: false_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(1, lhs.const_ty().unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(false_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(0, lhs.const_ty().unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                BinaryOperator::Or => {
                    let dst_name = Identifier::new("or_result");
                    let dst = Value::Var(dst_name.clone());
                    get_symbols().insert(
                        dst_name,
                        Symbol {
                            attributes: IdentifierAttributes::Local,
                            ty: Type::Int,
                        },
                    );
                    let true_label = Identifier::new("or_true");
                    let end_label = Identifier::new("or_end");

                    let lhs = lhs.lower(instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: lhs.clone(),
                        target: true_label.clone(),
                    });

                    let rhs = rhs.lower(instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: rhs,
                        target: true_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(0, lhs.const_ty().unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(true_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(1, lhs.const_ty().unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                op => unreachable!("operator {op:?} cannot appear here"),
            },
            ExprKind::Prefix(op, expr) => {
                assert!(
                    expr.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let expr = expr.lower(instructions);

                instructions.push(Instruction::Binary {
                    operator: ir::BinaryOperator::Add,
                    lhs: expr.clone(),
                    rhs: Value::Constant(op.n(expr.const_ty().unwrap())),
                    dst: expr.clone(),
                });

                expr
            }
            ExprKind::Postfix(op, expr) => {
                assert!(
                    expr.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let old_value_name = Identifier::new("postfix_old_value");
                let old_value = Value::Var(old_value_name.clone());
                get_symbols().insert(
                    old_value_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: expr.ty.clone(),
                    },
                );
                let expr = expr.lower(instructions);

                instructions.push(Instruction::Copy {
                    src: expr.clone(),
                    dst: old_value.clone(),
                });

                instructions.push(Instruction::Binary {
                    operator: ir::BinaryOperator::Add,
                    lhs: expr.clone(),
                    rhs: Value::Constant(op.n(expr.const_ty().unwrap())),
                    dst: expr.clone(),
                });

                old_value
            }
            ExprKind::Conditional(cond, if_true, if_false) => {
                let cond = cond.lower(instructions);
                let else_label = Identifier::new("conditional_else");
                let end_label = Identifier::new("conditional_end");
                let result_name = Identifier::new("conditional_result");
                let result = Value::Var(result_name.clone());
                get_symbols().insert(
                    result_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: if_true.ty.clone(),
                    },
                );

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
            ExprKind::FunctionCall { name, args } => {
                let args: Vec<_> = args.into_iter().map(|a| a.lower(instructions)).collect();
                let result_name = Identifier::new(format!("{}_result", name.1));
                let result = Value::Var(result_name.clone());

                let return_type = match &get_symbols().get(&name).unwrap().ty {
                    Type::Function(f) => f.return_type.clone(),
                    _ => unreachable!("call target must have function type"),
                };

                get_symbols().insert(
                    result_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: return_type,
                    },
                );

                instructions.push(Instruction::FunctionCall {
                    name,
                    args,
                    dst: result.clone(),
                });

                result
            }
            ExprKind::Cast { target_type, expr } => {
                let result = expr.lower(instructions);

                if target_type == result.ty() {
                    return result;
                }

                let dst_name = Identifier::new("cast_tmp");
                get_symbols().insert(
                    dst_name.clone(),
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: target_type.clone(),
                    },
                );
                let dst = Value::Var(dst_name);

                if target_type == Type::Long {
                    instructions.push(Instruction::SignExtend {
                        src: result,
                        dst: dst.clone(),
                    });
                } else if target_type == Type::Int {
                    instructions.push(Instruction::Truncate {
                        src: result,
                        dst: dst.clone(),
                    });
                }

                dst
            }
        }
    }
}

impl Constant {
    pub fn lower(self) -> ir::Value {
        match self {
            Constant::Int(i) => ir::Value::Constant(Constant::Int(i)),
            Constant::Long(l) => ir::Value::Constant(Constant::Long(l)),
        }
    }
}
