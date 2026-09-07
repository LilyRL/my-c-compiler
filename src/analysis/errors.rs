use crate::{analysis::typechecking::Type, diagnostics::Span, parser::Constant};

#[derive(Debug)]
pub enum SemanticError {
    InvalidLValue {
        expr: String,
        span: Span,
    },
    VariableRedeclaration {
        name: String,
        span: Span,
    },
    FunctionRedeclaration {
        name: String,
        span: Span,
    },
    UndeclaredVariable {
        name: String,
        span: Span,
    },
    UndeclaredFunction {
        name: String,
        span: Span,
    },
    BreakOutsideBreakable {
        span: Span,
    },
    ContinueOutsideLoop {
        span: Span,
    },
    DuplicateSwitchCase {
        value: Constant,
        span: Span,
    },
    DuplicateDefaultSwitchCase {
        span: Span,
    },
    CaseOutsideSwitch {
        span: Span,
    },
    IncompatibleFunctionDeclarations {
        span: Span,
        type_a: Type,
        type_b: Type,
    },
    FunctionRedefinition {
        name: String,
        span: Span,
    },
    WrongNumberOfArguements {
        span: Span,
        expected: usize,
        found: usize,
    },
    VariableUsedAsFunction {
        span: Span,
        name: String,
    },
    FunctionUsedAsVariable {
        span: Span,
        name: String,
    },
    UndeclaredGotoTarget {
        span: Span,
        name: String,
    },
    DuplicateLabel {
        span: Span,
        name: String,
    },
    NestedFunctionDeclaration {
        span: Span,
    },
    NestedFunction {
        span: Span,
        name: String,
    },
}

impl SemanticError {
    pub fn span(&self) -> &Span {
        match self {
            Self::InvalidLValue { span, .. }
            | Self::VariableRedeclaration { span, .. }
            | Self::UndeclaredVariable { span, .. }
            | Self::BreakOutsideBreakable { span }
            | Self::ContinueOutsideLoop { span }
            | Self::DuplicateSwitchCase { span, .. }
            | Self::DuplicateDefaultSwitchCase { span }
            | Self::CaseOutsideSwitch { span }
            | Self::FunctionRedeclaration { span, .. }
            | Self::IncompatibleFunctionDeclarations { span, .. }
            | Self::FunctionRedefinition { span, .. }
            | Self::WrongNumberOfArguements { span, .. }
            | Self::VariableUsedAsFunction { span, .. }
            | Self::FunctionUsedAsVariable { span, .. }
            | Self::UndeclaredGotoTarget { span, .. }
            | Self::DuplicateLabel { span, .. }
            | Self::NestedFunctionDeclaration { span, .. }
            | Self::NestedFunction { span, .. }
            | Self::UndeclaredFunction { span, .. } => span,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::InvalidLValue { expr, .. } => format!("invalid assignment target '{expr}'"),
            Self::VariableRedeclaration { name, .. } => {
                format!("redeclaration of variable '{name}'")
            }
            Self::UndeclaredVariable { name, .. } => format!("undeclared variable '{name}'"),
            Self::BreakOutsideBreakable { .. } => "'break' outside of a loop or switch".to_string(),
            Self::ContinueOutsideLoop { .. } => "'continue' outside of a loop".to_string(),
            Self::DuplicateSwitchCase { value, .. } => {
                format!("duplicate case value {}", value.clone().i32())
            }
            Self::DuplicateDefaultSwitchCase { .. } => {
                "multiple 'default' cases in one switch".to_string()
            }
            Self::CaseOutsideSwitch { .. } => {
                "'case'/'default' label outside of a switch".to_string()
            }
            Self::FunctionRedeclaration { name, .. } => {
                format!("redeclaration of function '{name}'")
            }
            Self::UndeclaredFunction { name, .. } => format!("undeclared function '{name}'"),
            Self::IncompatibleFunctionDeclarations { type_a, type_b, .. } => {
                format!("incompatible function declarations: '{type_a}' vs '{type_b}'")
            }
            Self::FunctionRedefinition { name, .. } => format!("redefinition of function '{name}'"),
            Self::WrongNumberOfArguements {
                expected, found, ..
            } => {
                format!("wrong number of arguments: expected {expected}, found {found}")
            }
            Self::VariableUsedAsFunction { name, .. } => {
                format!("variable '{name}' used as a function")
            }
            Self::FunctionUsedAsVariable { name, .. } => {
                format!("function '{name}' used as a variable")
            }
            Self::UndeclaredGotoTarget { name, .. } => {
                format!("undeclared goto target '{name}'")
            }
            Self::DuplicateLabel { name, .. } => {
                format!("duplicate label '{name}'")
            }
            Self::NestedFunctionDeclaration { .. } => {
                "functions can only be declared at the top level".to_string()
            }
            Self::NestedFunction { name, .. } => {
                format!("function '{name}' cannot be declared inside another function")
            }
        }
    }
}
