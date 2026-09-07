use super::*;
use std::fmt::Display;

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ExprKind::Var(i) => write!(f, "{}", i.0),
            ExprKind::Constant(c) => match c {
                Constant::Int(i) => write!(f, "{}", i),
            },
            ExprKind::Unary { operator, expr } => write!(f, "({} {})", operator.symbol(), expr),
            ExprKind::Binary { operator, lhs, rhs } => {
                write!(f, "({} {} {})", lhs, operator.symbol(), rhs)
            }
            ExprKind::Assignment(lhs, rhs) => write!(f, "({} = {})", lhs, rhs),
            ExprKind::CompoundAssign { operator, lhs, rhs } => {
                write!(f, "({} {} {})", lhs, operator.symbol(), rhs)
            }
            ExprKind::Prefix(inc_dec, expr) => write!(f, "({}{})", inc_dec.symbol(), expr),
            ExprKind::Postfix(inc_dec, expr) => write!(f, "({}{})", expr, inc_dec.symbol()),
            ExprKind::Conditional(cond, if_true, if_false) => {
                write!(f, "({} ? {} : {})", cond, if_true, if_false)
            }
            ExprKind::FunctionCall { name, args } => {
                let args_str = args
                    .iter()
                    .map(|arg| format!("{}", arg))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}({})", name.0, args_str)
            }
        }
    }
}
