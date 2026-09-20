pub use data::*;
mod data;
mod lowering;

use std::fmt::Display;

use crate::{analysis::StaticInit, parser::Constant};

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignExtend { src, dst } => write!(f, "\t{} = sign_extend({});", dst, src),
            Self::Truncate { src, dst } => write!(f, "\t{} = truncate({});", dst, src),
            Self::Return(value) => write!(f, "\treturn {};", value),
            Self::Unary { operator, src, dst } => {
                let op_str = operator.to_str();
                write!(f, "\t{} = {}{};", dst, op_str, src)
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let op_str = operator.to_str();
                write!(f, "\t{} = {} {} {};", dst, lhs, op_str, rhs)
            }
            Self::Copy { src, dst } => write!(f, "\t{} = {};", dst, src),
            Self::Jump(target) => write!(f, "\tgoto {};", target.0),
            Self::JumpIfZero { condition, target } => {
                write!(f, "\tjz {} => {}", condition, target.0)
            }
            Self::JumpNotZero { condition, target } => {
                write!(f, "\tjnz {} => {}", condition, target.0)
            }
            Self::Label(label) => write!(f, "{}:", label.0),
            Self::Comment(c) => write!(f, "\t# {}", c),
            Self::FunctionCall { name, args, dst } => {
                let args_str = args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "\t{} = {}({});", dst, name.0, args_str)
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(c) => write!(f, "{}", c),
            Self::Var(v) => write!(f, "{}", v.0),
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for func in self.0.iter() {
            write!(f, "{}\n\n", func)?;
        }

        Ok(())
    }
}

impl Display for TopLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::F(func) => write!(f, "{}", func),
            Self::V(var) => write!(f, "{}", var),
        }
    }
}

impl Display for StaticVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "static {} = {};", self.name.0, self.init)
    }
}

impl Display for StaticInit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{}", i),
            Self::Long(l) => write!(f, "{}", l),
        }
    }
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{}", i),
            Self::Long(l) => write!(f, "{}", l),
        }
    }
}

impl Display for FunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {}() {{\n", self.name.0)?;
        for instruction in &self.body {
            write!(f, "{}\n", instruction)?;
        }
        write!(f, "}}")
    }
}

impl UnaryOperator {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::BitwiseNot => "~",
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

impl BinaryOperator {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::BitwiseAnd => "&",
            Self::BitwiseXor => "^",
            Self::BitwiseOr => "|",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::LessEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterEqual => ">=",
        }
    }
}
