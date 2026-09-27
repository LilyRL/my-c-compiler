use super::*;

pub enum ConstExpr {
    Constant(Constant),
    Unary {
        operator: UnaryOperator,
        expr: Box<ConstExpr>,
    },
    Binary {
        operator: BinaryOperator,
        lhs: Box<ConstExpr>,
        rhs: Box<ConstExpr>,
    },
    Conditional(Box<ConstExpr>, Box<ConstExpr>, Box<ConstExpr>),
    Cast {
        target_type: ConstantType,
        expr: Box<ConstExpr>,
    },
}

impl ConstExpr {
    pub fn eval(self) -> Result<Constant, &'static str> {
        match self {
            Self::Constant(c) => Ok(c),
            Self::Unary { operator, expr } => {
                let c = expr.eval();

                match c? {
                    Constant::Int(n) => Ok(Constant::Int(match operator {
                        UnaryOperator::BitwiseNot => !n,
                        UnaryOperator::Not => !(n != 0) as i32,
                        UnaryOperator::Negate => -n,
                    })),
                    Constant::Long(n) => Ok(match operator {
                        UnaryOperator::BitwiseNot => Constant::Long(!n),
                        UnaryOperator::Not => Constant::Int(!(n != 0) as i32),
                        UnaryOperator::Negate => Constant::Long(-n),
                    }),
                    Constant::UInt(n) => match operator {
                        UnaryOperator::BitwiseNot => Ok(Constant::UInt(!n)),
                        UnaryOperator::Not => Ok(Constant::Int(!(n != 0) as i32)),
                        UnaryOperator::Negate => Err("cannot negate unsigned integer"),
                    },
                    Constant::ULong(n) => match operator {
                        UnaryOperator::BitwiseNot => Ok(Constant::ULong(!n)),
                        UnaryOperator::Not => Ok(Constant::Int(!(n != 0) as i32)),
                        UnaryOperator::Negate => Err("cannot negate unsigned long"),
                    },
                }
            }
            Self::Cast { target_type, expr } => Ok(expr.eval()?.cast(target_type)),
            Self::Binary { operator, lhs, rhs } => {
                let lhs = lhs.eval()?;
                let rhs = rhs.eval()?;

                Ok(match lhs.to_common_pair(rhs) {
                    (Constant::Int(lhs), Constant::Int(rhs)) => match operator {
                        BinaryOperator::Add => Constant::Int(lhs + rhs),
                        BinaryOperator::Subtract => Constant::Int(lhs - rhs),
                        BinaryOperator::Multiply => Constant::Int(lhs * rhs),
                        BinaryOperator::Divide => Constant::Int(lhs / rhs),
                        BinaryOperator::Remainder => Constant::Int(lhs % rhs),
                        BinaryOperator::BitwiseAnd => Constant::Int(lhs & rhs),
                        BinaryOperator::BitwiseXor => Constant::Int(lhs ^ rhs),
                        BinaryOperator::BitwiseOr => Constant::Int(lhs | rhs),
                        BinaryOperator::LeftShift => Constant::Int(lhs << rhs),
                        BinaryOperator::RightShift => Constant::Int(lhs >> rhs),
                        BinaryOperator::And => Constant::Int(((lhs != 0) && (rhs != 0)) as i32),
                        BinaryOperator::Or => Constant::Int(((lhs != 0) || (rhs != 0)) as i32),
                        BinaryOperator::Equal => Constant::Int((lhs == rhs) as i32),
                        BinaryOperator::NotEqual => Constant::Int((lhs != rhs) as i32),
                        BinaryOperator::LessThan => Constant::Int((lhs < rhs) as i32),
                        BinaryOperator::LessEqual => Constant::Int((lhs <= rhs) as i32),
                        BinaryOperator::GreaterThan => Constant::Int((lhs > rhs) as i32),
                        BinaryOperator::GreaterEqual => Constant::Int((lhs >= rhs) as i32),
                        BinaryOperator::Assign
                        | BinaryOperator::AddAssign
                        | BinaryOperator::SubtractAssign
                        | BinaryOperator::MultiplyAssign
                        | BinaryOperator::DivideAssign
                        | BinaryOperator::RemainderAssign
                        | BinaryOperator::BitwiseAndAssign
                        | BinaryOperator::BitwiseXorAssign
                        | BinaryOperator::BitwiseOrAssign
                        | BinaryOperator::LeftShiftAssign
                        | BinaryOperator::RightShiftAssign => unreachable!(),
                    },
                    (Constant::Long(lhs), Constant::Long(rhs)) => match operator {
                        BinaryOperator::Add => Constant::Long(lhs + rhs),
                        BinaryOperator::Subtract => Constant::Long(lhs - rhs),
                        BinaryOperator::Multiply => Constant::Long(lhs * rhs),
                        BinaryOperator::Divide => Constant::Long(lhs / rhs),
                        BinaryOperator::Remainder => Constant::Long(lhs % rhs),
                        BinaryOperator::BitwiseAnd => Constant::Long(lhs & rhs),
                        BinaryOperator::BitwiseXor => Constant::Long(lhs ^ rhs),
                        BinaryOperator::BitwiseOr => Constant::Long(lhs | rhs),
                        BinaryOperator::LeftShift => Constant::Long(lhs << rhs),
                        BinaryOperator::RightShift => Constant::Long(lhs >> rhs),
                        BinaryOperator::And => Constant::Int(((lhs != 0) && (rhs != 0)) as i32),
                        BinaryOperator::Or => Constant::Int(((lhs != 0) || (rhs != 0)) as i32),
                        BinaryOperator::Equal => Constant::Int((lhs == rhs) as i32),
                        BinaryOperator::NotEqual => Constant::Int((lhs != rhs) as i32),
                        BinaryOperator::LessThan => Constant::Int((lhs < rhs) as i32),
                        BinaryOperator::LessEqual => Constant::Int((lhs <= rhs) as i32),
                        BinaryOperator::GreaterThan => Constant::Int((lhs > rhs) as i32),
                        BinaryOperator::GreaterEqual => Constant::Int((lhs >= rhs) as i32),
                        BinaryOperator::Assign
                        | BinaryOperator::AddAssign
                        | BinaryOperator::SubtractAssign
                        | BinaryOperator::MultiplyAssign
                        | BinaryOperator::DivideAssign
                        | BinaryOperator::RemainderAssign
                        | BinaryOperator::BitwiseAndAssign
                        | BinaryOperator::BitwiseXorAssign
                        | BinaryOperator::BitwiseOrAssign
                        | BinaryOperator::LeftShiftAssign
                        | BinaryOperator::RightShiftAssign => unreachable!(),
                    },
                    _ => unreachable!(),
                })
            }
            Self::Conditional(cond, then, else_) => {
                let cond = cond.eval()?;

                if cond.is_zero() {
                    else_.eval()
                } else {
                    then.eval()
                }
            }
        }
    }
}

impl Expression {
    pub fn eval(&self) -> Result<Constant, &'static str> {
        self.to_constant()?.eval()
    }

    pub fn to_constant(&self) -> Result<ConstExpr, &'static str> {
        let res = match &self.kind {
            ExprKind::Var(_)
            | ExprKind::CompoundAssign { .. }
            | ExprKind::Assignment(_, _)
            | ExprKind::Prefix(_, _)
            | ExprKind::Postfix(_, _)
            | ExprKind::FunctionCall { .. } => None,
            ExprKind::Constant(c) => Some(ConstExpr::Constant(*c)),
            ExprKind::Cast { target_type, expr } => {
                let expr = expr.to_constant()?;
                let ty = match target_type {
                    Type::Int => ConstantType::Int,
                    Type::Long => ConstantType::Long,
                    Type::UInt => ConstantType::UInt,
                    Type::ULong => ConstantType::ULong,
                    Type::Function(_) => return Err("non-constant expression"),
                };
                Some(ConstExpr::Cast {
                    target_type: ty,
                    expr: Box::new(expr),
                })
            }
            ExprKind::Unary { operator, expr } => {
                let expr = expr.to_constant()?;
                Some(ConstExpr::Unary {
                    operator: *operator,
                    expr: Box::new(expr),
                })
            }
            ExprKind::Binary { operator, lhs, rhs } => {
                if operator.is_assign() || operator.is_compound_assign() {
                    return Err("non-constant expression");
                }

                let lhs = lhs.to_constant()?;
                let rhs = rhs.to_constant()?;
                Some(ConstExpr::Binary {
                    operator: *operator,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                })
            }
            ExprKind::Conditional(cond, then, else_) => {
                let cond = cond.to_constant()?;
                let then = then.to_constant()?;
                let else_ = else_.to_constant()?;
                Some(ConstExpr::Conditional(
                    Box::new(cond),
                    Box::new(then),
                    Box::new(else_),
                ))
            }
        };

        res.ok_or("non-constant expression")
    }
}
