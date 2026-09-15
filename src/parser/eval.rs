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
}

impl ConstExpr {
    pub fn eval(self) -> Constant {
        match self {
            Self::Constant(c) => c,
            Self::Unary { operator, expr } => {
                let c = expr.eval();

                match c {
                    Constant::Int(n) => Constant::Int(match operator {
                        UnaryOperator::BitwiseNot => !n,
                        UnaryOperator::Not => !(n != 0) as i32,
                        UnaryOperator::Negate => -n,
                    }),
                }
            }
            Self::Binary { operator, lhs, rhs } => {
                let lhs = lhs.eval();
                let rhs = rhs.eval();

                match (lhs, rhs) {
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
                }
            }
            Self::Conditional(cond, then, else_) => {
                let cond = cond.eval();

                match cond {
                    Constant::Int(0) => else_.eval(),
                    Constant::Int(_) => then.eval(),
                }
            }
        }
    }
}

impl Expression {
    pub fn eval(&self) -> Option<Constant> {
        self.to_constant().map(|c| c.eval())
    }

    pub fn to_constant(&self) -> Option<ConstExpr> {
        match &self.kind {
            ExprKind::Var(_)
            | ExprKind::CompoundAssign { .. }
            | ExprKind::Assignment(_, _)
            | ExprKind::Prefix(_, _)
            | ExprKind::Postfix(_, _)
            | ExprKind::FunctionCall { .. } => None,
            ExprKind::Constant(c) => Some(ConstExpr::Constant(*c)),
            ExprKind::Unary { operator, expr } => {
                let expr = expr.to_constant()?;
                Some(ConstExpr::Unary {
                    operator: *operator,
                    expr: Box::new(expr),
                })
            }
            ExprKind::Binary { operator, lhs, rhs } => {
                if operator.is_assign() || operator.is_compound_assign() {
                    return None;
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
        }
    }
}
