use super::*;
use crate::ir;
use std::fmt::Display;

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for FunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut block_str = String::new();
        for item in &self.block {
            block_str.push_str(&format!("{}", item));
        }
        write!(f, "int {}(void) {{\n{}}}", self.name.0, block_str)
    }
}

impl Display for BlockItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stmt(stmt) => write!(f, "{}", stmt),
            Self::Decl(Declaration {
                name,
                init: Some(init),
            }) => write!(f, "int {} = {};", name.0, init),
            Self::Decl(Declaration { name, init: None }) => write!(f, "    int {};\n", name.0),
        }
    }
}

impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return(e) => write!(f, "    return {};", e),
            Self::Expression(e) => write!(f, "    {};", e),
            Self::If { cond, then, else_ } => {
                if let Some(else_stmt) = else_ {
                    write!(
                        f,
                        "    if ({}) {{ {} }} else {{ {} }}",
                        cond, then, else_stmt
                    )
                } else {
                    write!(f, "    if ({}) {{ {} }}", cond, then)
                }
            }
            Self::Null => write!(f, "    ; // null statement"),
            Self::Label(l) => write!(f, "{}:", l.0),
            Self::Goto(l) => write!(f, "    goto {};", l.0),
            Self::Compound(block) => {
                let mut block_str = String::new();
                for item in block.iter() {
                    block_str.push_str(&format!("    {}\n", item));
                }
                write!(f, "    {{\n{}    }}\n", block_str)
            }
        }
    }
}

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

impl Constant {
    pub fn lower(self) -> ir::Value {
        match self {
            Constant::Int(i) => ir::Value::Constant(i),
        }
    }
}
