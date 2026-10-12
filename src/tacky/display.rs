use std::fmt::Display;

use crate::core::Symbols;

use super::data::*;

impl Instruction {
    pub fn format(&self, symbols: &Symbols) -> String {
        match self {
            Self::ZeroExtend { src, dst } => {
                format!("\t{} {} = zero_extend({});", src.ty(symbols), dst, src)
            }
            Self::SignExtend { src, dst } => {
                format!("\t{} {} = sign_extend({});", src.ty(symbols), dst, src)
            }
            Self::Truncate { src, dst } => {
                format!("\t{} {} = truncate({});", src.ty(symbols), dst, src)
            }
            Self::Return(value) => format!("\treturn {};", value),
            Self::Unary { operator, src, dst } => {
                let op_str = operator.to_str();
                format!("\t{} {} = {}{};", src.ty(symbols), dst, op_str, src)
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let op_str = operator.to_str();
                format!(
                    "\t{} {} = {} {} {};",
                    lhs.ty(symbols),
                    dst,
                    lhs,
                    op_str,
                    rhs
                )
            }
            Self::Copy { src, dst } => format!("\t{} {} = {};", src.ty(symbols), dst, src),
            Self::Jump(target) => format!("\tgoto {};", target.0),
            Self::JumpIfZero { condition, target } => {
                format!("\tjz {} => {}", condition, target.0)
            }
            Self::JumpNotZero { condition, target } => {
                format!("\tjnz {} => {}", condition, target.0)
            }
            Self::Label(label) => format!("{}:", label.0),
            Self::Comment(c) => format!("\t# {}", c),
            Self::FunctionCall { name, args, dst } => {
                let args_str = args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("\t{} {} = {}({});", dst.ty(symbols), dst, name.0, args_str)
            }
            Self::DoubleToInt { src, dst } => {
                format!("\t{} {} = double_to_int({});", src.ty(symbols), dst, src)
            }
            Self::DoubleToUInt { src, dst } => {
                format!("\t{} {} = double_to_uint({});", src.ty(symbols), dst, src)
            }
            Self::DoubleToFloat { src, dst } => {
                format!("\t{} {} = double_to_float({});", src.ty(symbols), dst, src)
            }
            Self::FloatToDouble { src, dst } => {
                format!("\t{} {} = float_to_double({});", src.ty(symbols), dst, src)
            }
            Self::IntToDouble { src, dst } => {
                format!("\t{} {} = int_to_double({});", src.ty(symbols), dst, src)
            }
            Self::UIntToDouble { src, dst } => {
                format!("\t{} {} = uint_to_double({});", src.ty(symbols), dst, src)
            }
            Self::FloatToInt { src, dst } => {
                format!("\t{} {} = float_to_int({});", src.ty(symbols), dst, src)
            }
            Self::FloatToUInt { src, dst } => {
                format!("\t{} {} = float_to_uint({});", src.ty(symbols), dst, src)
            }
            Self::IntToFloat { src, dst } => {
                format!("\t{} {} = int_to_float({});", src.ty(symbols), dst, src)
            }
            Self::UIntToFloat { src, dst } => {
                format!("\t{} {} = uint_to_float({});", src.ty(symbols), dst, src)
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

impl Program {
    pub fn format(&self, symbols: &Symbols) -> String {
        let mut result = String::new();
        for func in self.0.iter() {
            result.push_str(&format!("{}\n\n", func.format(symbols)));
        }
        result
    }
}

impl TopLevel {
    pub fn format(&self, symbols: &Symbols) -> String {
        match self {
            Self::F(func) => func.format(symbols),
            Self::V(var) => format!("{}", var),
        }
    }
}

impl Display for StaticVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "static {} = {};", self.name.0, self.init)
    }
}

impl FunctionDefinition {
    fn format(&self, symbols: &Symbols) -> String {
        let params_str = self
            .params
            .iter()
            .map(|param| format!("{} {}", param.ty, param.name.0))
            .collect::<Vec<String>>()
            .join(", ");
        let mut result = format!("fn {}({}) {{\n", self.name.0, params_str);
        for instruction in &self.body {
            result.push_str(&format!("{}\n", instruction.format(symbols)));
        }
        result.push('}');
        result
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
