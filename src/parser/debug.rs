use super::*;
use crate::ir;
use std::fmt::Display;

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(i) => write!(f, "{}", i.0),
            Self::Constant(c) => match c {
                Constant::Int(i) => write!(f, "{}", i),
            },
            Self::Unary { operator, expr } => write!(f, "({} {})", operator.symbol(), expr),
            Self::Binary { operator, lhs, rhs } => {
                write!(f, "({} {} {})", lhs, operator.symbol(), rhs)
            }
            Self::Assignment(lhs, rhs) => write!(f, "({} = {})", lhs, rhs),
            Self::CompoundAssign { operator, lhs, rhs } => {
                write!(f, "({} {} {})", lhs, operator.symbol(), rhs)
            }
            Self::Prefix(inc_dec, expr) => write!(f, "({}{})", inc_dec.symbol(), expr),
            Self::Postfix(inc_dec, expr) => write!(f, "({}{})", expr, inc_dec.symbol()),
            Self::Conditional(cond, if_true, if_false) => {
                write!(f, "({} ? {} : {})", cond, if_true, if_false)
            }
        }
    }
}
