use std::{collections::HashMap, fmt::Display};

use strum::EnumIs;

use crate::{
    analysis::declarations::Scope,
    diagnostics::Diagnostics,
    parser::{
        BlockItem, Constant, Declaration, ExprKind, Expression, ForInit, FunctionDeclaration,
        Identifier, Program, Statement, StmtKind, Switch, VariableDeclaration,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Function(FunctionType),
}

#[derive(Clone, Debug, EnumIs)]
pub enum IdentifierAttributes {
    Function { defined: bool, global: bool },
    Static { init: InitialValue, global: bool },
    Local,
}

impl IdentifierAttributes {
    pub fn global(&self) -> bool {
        match self {
            IdentifierAttributes::Function { global, .. } => *global,
            IdentifierAttributes::Static { global, .. } => *global,
            IdentifierAttributes::Local => false,
        }
    }

    pub fn initial_value(&self) -> Option<InitialValue> {
        match self {
            IdentifierAttributes::Function { .. } => None,
            IdentifierAttributes::Static { init, .. } => Some(init.clone()),
            IdentifierAttributes::Local => None,
        }
    }
}

#[derive(Clone, Debug, EnumIs)]
pub enum InitialValue {
    Tentitive,
    Constant(Constant),
    None,
}

#[derive(Debug)]
pub struct Symbol {
    pub attributes: IdentifierAttributes,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    num_parameters: usize,
    defined: bool,
}

pub type Symbols = HashMap<Identifier, Symbol>;
sge_global::global!(Symbols, symbols);

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Function(FunctionType { num_parameters, .. }) => {
                write!(f, "int(")?;

                let mut n = *num_parameters;

                if n != 0 {
                    write!(f, "int")?;
                    n -= 1;

                    for _ in 0..n {
                        write!(f, ", int")?;
                    }
                }

                write!(f, ")")
            }
        }
    }
}

pub fn check_all_types(program: &Program, diagnostics: &mut Diagnostics) {
    let mut symbols = Symbols::new();

    for declaration in &program.0 {
        match declaration {
            Declaration::Var(decl) => {
                check_variable_declaration(decl, &mut symbols, diagnostics, Scope::Global)
            }
            Declaration::Func(decl) => check_function_declaration(decl, &mut symbols, diagnostics),
        }
    }

    set_symbols(symbols);
}

fn check_variable_declaration(
    decl: &VariableDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
    scope: Scope,
) {
    match scope {
        Scope::Global => check_global_variable_declaration(decl, symbols, diagnostics),
        Scope::Local => check_local_variable(decl, symbols, diagnostics),
    }
}

fn check_local_variable(
    decl: &VariableDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) {
    if decl.storage_class.is_extern() {
        if decl.init.is_some() {
            diagnostics.analysis_error(
                decl.span.clone(),
                "initializer on local extern variable declaration",
            );
        }

        if let Some(old_decl) = symbols.get(&decl.name) {
            if old_decl.ty != Type::Int {
                diagnostics.analysis_error(decl.span.clone(), "function redeclared as variable");
            }
        } else {
            symbols.insert(
                decl.name.clone(),
                Symbol {
                    attributes: IdentifierAttributes::Static {
                        init: InitialValue::None,
                        global: true,
                    },
                    ty: Type::Int,
                },
            );
        }
    } else if decl.storage_class.is_static() {
        let initial_value;
        if let Some(Some(constant)) = decl.init.as_ref().map(|i| i.eval()) {
            initial_value = InitialValue::Constant(constant.clone());
        } else if decl.init.is_none() {
            initial_value = InitialValue::Tentitive;
        } else {
            diagnostics.analysis_error(decl.span.clone(), "initializer is not a constant");
            return;
        }

        symbols.insert(
            decl.name.clone(),
            Symbol {
                attributes: IdentifierAttributes::Static {
                    init: initial_value,
                    global: false,
                },
                ty: Type::Int,
            },
        );
    } else {
        symbols.insert(
            decl.name.clone(),
            Symbol {
                attributes: IdentifierAttributes::Local,
                ty: Type::Int,
            },
        );

        if let Some(init) = &decl.init {
            check_expression(init, symbols, diagnostics);
        }
    }
}

fn check_global_variable_declaration(
    decl: &VariableDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) {
    let mut initial_value;
    if let Some(init) = &decl.init {
        if let Some(constant) = init.eval() {
            initial_value = InitialValue::Constant(constant);
        } else {
            diagnostics.analysis_error(decl.span.clone(), "initializer is not a constant");
            return;
        }
    } else if decl.storage_class.is_extern() {
        initial_value = InitialValue::None;
    } else {
        initial_value = InitialValue::Tentitive;
    }

    let mut is_global = !decl.storage_class.is_static();

    if let Some(old_decl) = symbols.get(&decl.name) {
        if old_decl.ty != Type::Int {
            diagnostics.analysis_error(
                decl.span.clone(),
                format!(
                    "'{}' redeclared as a variable but was previously a function",
                    decl.name.1
                ),
            );
        }

        if decl.storage_class.is_extern() {
            is_global = old_decl.attributes.global();
        } else if old_decl.attributes.global() != is_global {
            diagnostics.analysis_error(
                decl.span.clone(),
                format!("conflicting linkage for variable '{}'", decl.name.1),
            );
        }

        if let Some(old_init) = old_decl.attributes.initial_value() {
            if old_init.is_constant() {
                if initial_value.is_constant() {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!("conflicting file-scope definitions for '{}'", decl.name.1),
                    );
                } else {
                    initial_value = old_init;
                }
            } else if !initial_value.is_constant() && old_init.is_tentitive() {
                initial_value = InitialValue::Tentitive;
            }
        }
    }

    let attributes = IdentifierAttributes::Static {
        init: initial_value,
        global: is_global,
    };

    symbols.insert(
        decl.name.clone(),
        Symbol {
            attributes,
            ty: Type::Int,
        },
    );
}

fn check_function_declaration(
    decl: &FunctionDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) {
    let has_body = decl.body.is_some();
    let mut new = FunctionType {
        num_parameters: decl.params.len(),
        defined: has_body,
    };
    let mut is_global = !decl.storage_class.is_static();

    if let Some(old_decl) = symbols.get(&decl.name) {
        match &old_decl.ty {
            Type::Function(old) => {
                new.defined = old.defined || has_body;

                if old.num_parameters != new.num_parameters {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!(
                            "incompatible function declarations: '{}' vs '{}'",
                            Type::Function(old.clone()),
                            Type::Function(new.clone())
                        ),
                    );
                }

                if old.defined && has_body {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!("redefinition of function '{}'", decl.name.1),
                    );
                }

                if old_decl.attributes.global() && decl.storage_class.is_static() {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!(
                            "static declaration of '{}' follows non-static declaration",
                            decl.name.1
                        ),
                    );
                }

                is_global = old_decl.attributes.global();
            }
            _ => {
                diagnostics.analysis_error(
                    decl.span.clone(),
                    format!(
                        "'{}' redeclared as a function but was previously declared as a variable",
                        decl.name.1
                    ),
                );
            }
        }
    }

    let attributes = IdentifierAttributes::Function {
        defined: new.defined,
        global: is_global,
    };

    symbols.insert(
        decl.name.clone(),
        Symbol {
            attributes,
            ty: Type::Function(new),
        },
    );

    if let Some(body) = &decl.body {
        for param in &decl.params {
            symbols.insert(
                param.name.clone(),
                Symbol {
                    attributes: IdentifierAttributes::Local,
                    ty: Type::Int,
                },
            );
        }

        check_block(body, symbols, diagnostics);
    }
}

fn check_expression_inner(exp: &Expression, symbols: &mut Symbols, diagnostics: &mut Diagnostics) {
    match &exp.kind {
        ExprKind::FunctionCall { name, args } => {
            if let Some(symbol) = symbols.get(&name) {
                match symbol.ty {
                    Type::Function(FunctionType { num_parameters, .. }) => {
                        if num_parameters != args.len() {
                            diagnostics.analysis_error(
                                exp.span.clone(),
                                format!(
                                    "wrong number of arguments: expected {}, found {}",
                                    num_parameters,
                                    args.len()
                                ),
                            );
                        }
                    }
                    _ => {
                        diagnostics.analysis_error(
                            exp.span.clone(),
                            format!("variable '{}' used as a function", name.1),
                        );
                    }
                }
            } else {
                // already caught during identifier resolution
            }
        }
        ExprKind::Var(v) => {
            if let Some(ty) = symbols.get(&v) {
                match ty.ty {
                    Type::Function(_) => {
                        diagnostics.analysis_error(
                            exp.span.clone(),
                            format!("function '{}' used as a variable", v.1),
                        );
                    }
                    _ => {}
                }
            } else {
                // already caught during identifier resolution
            }
        }
        _ => (),
    }
}

fn check_expression(exp: &Expression, symbols: &mut Symbols, diagnostics: &mut Diagnostics) {
    let mut state = (symbols, diagnostics);
    exp.process_inner_expressions(&mut state, &|exp, (symbols, diagnostics)| {
        check_expression_inner(exp, symbols, diagnostics);
    });
}

fn check_block(block: &Vec<BlockItem>, symbols: &mut Symbols, diagnostics: &mut Diagnostics) {
    for item in block {
        match item {
            BlockItem::Stmt(s) => check_statement(s, symbols, diagnostics),
            BlockItem::Decl(decl) => match decl {
                Declaration::Var(v) => {
                    check_variable_declaration(v, symbols, diagnostics, Scope::Local)
                }
                Declaration::Func(f) => check_function_declaration(f, symbols, diagnostics),
            },
        }
    }
}

fn check_statement(stmt: &Statement, symbols: &mut Symbols, diagnostics: &mut Diagnostics) {
    match &stmt.kind {
        StmtKind::Compound(items) => {
            for item in items {
                match item {
                    BlockItem::Stmt(s) => check_statement(s, symbols, diagnostics),
                    BlockItem::Decl(d) => match d {
                        Declaration::Var(v) => {
                            check_variable_declaration(v, symbols, diagnostics, Scope::Local)
                        }
                        Declaration::Func(f) => check_function_declaration(f, symbols, diagnostics),
                    },
                }
            }
        }
        StmtKind::Switch(Switch { value, body, .. }) => {
            check_expression(value, symbols, diagnostics);
            check_statement(body, symbols, diagnostics);
        }
        StmtKind::If { cond, then, else_ } => {
            check_expression(cond, symbols, diagnostics);
            check_statement(then, symbols, diagnostics);
            if let Some(else_) = else_ {
                check_statement(else_, symbols, diagnostics);
            }
        }
        StmtKind::While { cond, body, .. } | StmtKind::DoWhile { body, cond, .. } => {
            check_expression(cond, symbols, diagnostics);
            check_statement(body, symbols, diagnostics);
        }
        StmtKind::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            match init {
                ForInit::Decl(d) => {
                    check_variable_declaration(d, symbols, diagnostics, Scope::Local)
                }
                ForInit::Expr(e) => check_expression(e, symbols, diagnostics),
                ForInit::None => {}
            }
            if let Some(condition) = condition {
                check_expression(condition, symbols, diagnostics);
            }
            if let Some(post) = post {
                check_expression(post, symbols, diagnostics);
            }
            check_statement(body, symbols, diagnostics);
        }
        StmtKind::Return(e) | StmtKind::Expression(e) => check_expression(e, symbols, diagnostics),
        StmtKind::Label(_, stmt)
        | StmtKind::Case { stmt, .. }
        | StmtKind::DefaultCase { stmt, .. } => check_statement(stmt, symbols, diagnostics),
        StmtKind::Null | StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Goto(_) => {}
    }
}
