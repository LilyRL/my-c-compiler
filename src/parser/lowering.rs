use super::*;
use crate::{
    analysis::{IdentifierAttributes, InitialValue, StaticInit, Symbol, Symbols},
    ir::{self, StaticVariable, TopLevel},
};
use ir::{Instruction, Value};

impl Program {
    pub fn lower(self, symbols: &mut Symbols) -> ir::Program {
        let mut definitions = self
            .0
            .into_iter()
            .flat_map(|d| match d {
                Declaration::Func(f) => f.lower(symbols).map(TopLevel::F),
                _ => None,
            })
            .collect::<Vec<_>>();

        definitions.extend(Self::lower_static_variables(symbols));

        ir::Program(definitions)
    }

    fn lower_static_variables(symbols: &Symbols) -> Vec<TopLevel> {
        let mut statics: Vec<_> = symbols
            .iter()
            .filter_map(|(name, entry)| {
                let IdentifierAttributes::Static { init, global } = &entry.attributes else {
                    return None;
                };

                let init = match init {
                    InitialValue::Constant(i) => *i,
                    InitialValue::Tentitive => match entry.ty {
                        Type::Int => StaticInit::Int(0),
                        Type::Long => StaticInit::Long(0),
                        Type::UInt => StaticInit::UInt(0),
                        Type::ULong => StaticInit::ULong(0),
                        _ => unreachable!(),
                    },
                    InitialValue::None => return None,
                };

                Some(TopLevel::V(StaticVariable {
                    name: name.clone(),
                    global: *global,
                    init,
                    ty: entry.ty.clone(),
                }))
            })
            .collect();

        // sort so the output is the same each time
        statics.sort_by(|a, b| match (a, b) {
            (TopLevel::V(a), TopLevel::V(b)) => a.name.cmp(&b.name),
            _ => unreachable!("only variables are produced here"),
        });

        statics
    }
}

impl FunctionDeclaration {
    pub fn lower(self, symbols: &mut Symbols) -> Option<ir::FunctionDefinition> {
        let mut instructions = Vec::new();

        let global = symbols
            .get(&self.name)
            .map(|s| s.attributes.global())
            .unwrap_or_else(|| !self.storage_class.is_static());

        for statement in self.body? {
            statement.lower(symbols, &mut instructions);
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
    pub fn lower(self, symbols: &mut Symbols, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Stmt(stmt) => stmt.lower(symbols, instructions),
            Self::Decl(decl) => decl.lower(symbols, instructions),
        }
    }
}

impl Declaration {
    pub fn lower(self, symbols: &mut Symbols, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Func(_) => {}
            Self::Var(var) => var.lower(symbols, instructions),
        }
    }
}

impl Statement {
    pub fn lower(self, symbols: &mut Symbols, instructions: &mut Vec<Instruction>) {
        match self.kind {
            StmtKind::Return(c) => {
                let dst = c.lower(symbols, instructions);
                instructions.push(Instruction::Return(dst));
            }
            StmtKind::Expression(exp) => {
                exp.lower(symbols, instructions);
            }
            StmtKind::If { cond, then, else_ } => {
                let cond = cond.lower(symbols, instructions);
                let end_label = Identifier::new("if_end");

                if let Some(else_) = else_ {
                    let else_label = Identifier::new("if_else");
                    instructions.push(Instruction::JumpIfZero {
                        condition: cond,
                        target: else_label.clone(),
                    });
                    then.lower(symbols, instructions);
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(else_label));
                    else_.lower(symbols, instructions);
                    instructions.push(Instruction::Label(end_label));
                } else {
                    instructions.push(Instruction::JumpIfZero {
                        condition: cond,
                        target: end_label.clone(),
                    });
                    then.lower(symbols, instructions);
                    instructions.push(Instruction::Label(end_label));
                }
            }
            StmtKind::DoWhile { body, cond, label } => {
                let start_label = label._start();
                let continue_label = label._continue();
                let break_label = label._break();

                instructions.push(Instruction::Label(start_label.clone()));
                body.lower(symbols, instructions);
                instructions.push(Instruction::Label(continue_label));
                let result = cond.lower(symbols, instructions);
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
                let result = cond.lower(symbols, instructions);
                instructions.push(Instruction::JumpIfZero {
                    condition: result,
                    target: break_label.clone(),
                });
                body.lower(symbols, instructions);
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
                init.lower(symbols, instructions);
                instructions.push(Instruction::Label(start_label.clone()));

                if let Some(condition) = condition {
                    instructions.push(Instruction::Comment("for loop condition"));
                    let cond_result = condition.lower(symbols, instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: cond_result,
                        target: break_label.clone(),
                    });
                } else {
                    instructions.push(Instruction::Comment("no for loop condition"));
                }

                instructions.push(Instruction::Comment("for loop body"));
                body.lower(symbols, instructions);
                instructions.push(Instruction::Label(continue_label));
                if let Some(post) = post {
                    post.lower(symbols, instructions);
                }
                instructions.push(Instruction::Jump(start_label));
                instructions.push(Instruction::Label(break_label));
            }
            StmtKind::Label(i, stmt) => {
                instructions.push(Instruction::Label(i));
                stmt.lower(symbols, instructions);
            }
            StmtKind::Goto(i) => instructions.push(Instruction::Jump(i)),
            StmtKind::Compound(block) => {
                for item in block {
                    item.lower(symbols, instructions);
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
                stmt.lower(symbols, instructions);
            }
            StmtKind::Switch(Switch {
                value,
                body,
                label,
                cases,
                default_case,
                ..
            }) => {
                let result = value.lower(symbols, instructions);
                let is_equal_name = Identifier::new("switch_case_is_equal");
                let is_equal = Value::Var(is_equal_name.clone());
                symbols.insert(
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

                body.lower(symbols, instructions);

                instructions.push(Instruction::Label(break_label));
            }
            StmtKind::Null => (),
        }
    }
}

impl VariableDeclaration {
    pub fn lower(self, symbols: &mut Symbols, instructions: &mut Vec<Instruction>) {
        if let Some(init) = self.init
            && self.storage_class.is_none()
        {
            let out = init.lower(symbols, instructions);
            let copy = Instruction::Copy {
                src: out,
                dst: Value::Var(self.name),
            };
            instructions.push(copy);
        }
    }
}

impl ForInit {
    pub fn lower(self, symbols: &mut Symbols, instructions: &mut Vec<Instruction>) {
        match self {
            Self::Decl(decl) => {
                decl.lower(symbols, instructions);
            }
            Self::Expr(expr) => {
                expr.lower(symbols, instructions);
            }
            Self::None => (),
        }
    }
}

impl Expression {
    pub fn lower(self, symbols: &mut Symbols, instructions: &mut Vec<Instruction>) -> Value {
        match self.kind {
            ExprKind::Assignment(lhs, rhs) => {
                assert!(
                    lhs.is_var(),
                    "LValues should have been verified to be valid before lowering to IR."
                );

                let v = lhs.as_var().cloned().unwrap();

                let result = rhs.lower(symbols, instructions);
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
                symbols.insert(
                    dst_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: expr.ty.clone(),
                    },
                );

                let src = expr.lower(symbols, instructions);

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
                symbols.insert(
                    dst_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: lhs.ty.clone(),
                    },
                );
                let lhs = lhs.lower(symbols, instructions);
                let rhs = rhs.lower(symbols, instructions);

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
                    symbols.insert(
                        dst_name,
                        Symbol {
                            attributes: IdentifierAttributes::Local,
                            ty: Type::Int,
                        },
                    );
                    let false_label = Identifier::new("and_false");
                    let end_label = Identifier::new("and_end");

                    let lhs = lhs.lower(symbols, instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: lhs.clone(),
                        target: false_label.clone(),
                    });

                    let rhs = rhs.lower(symbols, instructions);
                    instructions.push(Instruction::JumpIfZero {
                        condition: rhs,
                        target: false_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(1, lhs.const_ty(symbols).unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(false_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(0, lhs.const_ty(symbols).unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Label(end_label));

                    return dst;
                }
                BinaryOperator::Or => {
                    let dst_name = Identifier::new("or_result");
                    let dst = Value::Var(dst_name.clone());
                    symbols.insert(
                        dst_name,
                        Symbol {
                            attributes: IdentifierAttributes::Local,
                            ty: Type::Int,
                        },
                    );
                    let true_label = Identifier::new("or_true");
                    let end_label = Identifier::new("or_end");

                    let lhs = lhs.lower(symbols, instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: lhs.clone(),
                        target: true_label.clone(),
                    });

                    let rhs = rhs.lower(symbols, instructions);
                    instructions.push(Instruction::JumpNotZero {
                        condition: rhs,
                        target: true_label.clone(),
                    });

                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(0, lhs.const_ty(symbols).unwrap())),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::Jump(end_label.clone()));
                    instructions.push(Instruction::Label(true_label));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(Constant::from_int(1, lhs.const_ty(symbols).unwrap())),
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

                let expr = expr.lower(symbols, instructions);

                instructions.push(Instruction::Binary {
                    operator: op.lowered_operator().lower(),
                    lhs: expr.clone(),
                    rhs: Value::Constant(Constant::from_int(1, expr.const_ty(symbols).unwrap())),
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
                symbols.insert(
                    old_value_name,
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: expr.ty.clone(),
                    },
                );
                let expr = expr.lower(symbols, instructions);

                instructions.push(Instruction::Copy {
                    src: expr.clone(),
                    dst: old_value.clone(),
                });

                instructions.push(Instruction::Binary {
                    operator: op.lowered_operator().lower(),
                    lhs: expr.clone(),
                    rhs: Value::Constant(Constant::from_int(1, expr.const_ty(symbols).unwrap())),
                    dst: expr.clone(),
                });

                old_value
            }
            ExprKind::Conditional(cond, if_true, if_false) => {
                let cond = cond.lower(symbols, instructions);
                let else_label = Identifier::new("conditional_else");
                let end_label = Identifier::new("conditional_end");
                let result_name = Identifier::new("conditional_result");
                let result = Value::Var(result_name.clone());
                symbols.insert(
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

                let if_true = if_true.lower(symbols, instructions);
                instructions.push(Instruction::Copy {
                    src: if_true,
                    dst: result.clone(),
                });
                instructions.push(Instruction::Jump(end_label.clone()));

                instructions.push(Instruction::Label(else_label));
                let if_false = if_false.lower(symbols, instructions);
                instructions.push(Instruction::Copy {
                    src: if_false,
                    dst: result.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                result
            }
            ExprKind::FunctionCall { name, args } => {
                let args: Vec<_> = args
                    .into_iter()
                    .map(|a| a.lower(symbols, instructions))
                    .collect();
                let result_name = Identifier::new(format!("{}_result", name.1));
                let result = Value::Var(result_name.clone());

                let return_type = match &symbols.get(&name).unwrap().ty {
                    Type::Function(f) => f.return_type.clone(),
                    _ => unreachable!("call target must have function type"),
                };

                symbols.insert(
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
                let result = expr.lower(symbols, instructions);
                let src_type = result.ty(symbols);

                if target_type == src_type {
                    return result;
                }

                let dst_name = Identifier::new("cast_tmp");
                symbols.insert(
                    dst_name.clone(),
                    Symbol {
                        attributes: IdentifierAttributes::Local,
                        ty: target_type.clone(),
                    },
                );
                let dst = Value::Var(dst_name);

                let src_size = src_type.size_bytes();
                let target_size = target_type.size_bytes();

                if src_size == target_size {
                    instructions.push(Instruction::Copy {
                        src: result,
                        dst: dst.clone(),
                    });
                } else if src_size > target_size {
                    instructions.push(Instruction::Truncate {
                        src: result,
                        dst: dst.clone(),
                    });
                } else if src_type.is_signed() {
                    instructions.push(Instruction::SignExtend {
                        src: result,
                        dst: dst.clone(),
                    });
                } else {
                    instructions.push(Instruction::ZeroExtend {
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
            Constant::UInt(i) => ir::Value::Constant(Constant::UInt(i)),
            Constant::ULong(i) => ir::Value::Constant(Constant::ULong(i)),
        }
    }
}
