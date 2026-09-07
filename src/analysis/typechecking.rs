use std::{collections::HashMap, fmt::Display};

use crate::{
    analysis::errors::SemanticError,
    parser::{
        BlockItem, Declaration, ExprKind, Expression, FunctionDeclaration, Identifier, Program,
        Statement, StmtKind, VariableDeclaration,
    },
};

#[derive(Clone, Debug)]
pub enum Type {
    Int,
    Function(FunctionType),
}

#[derive(Clone, Debug)]
pub struct FunctionType {
    num_parameters: usize,
    defined: bool,
}

type Symbols = HashMap<Identifier, Type>;

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

pub fn check_all_types(program: &Program, errors: &mut Vec<SemanticError>) {
    let mut symbols = Symbols::new();
    for func in &program.0 {
        check_function_declaration(func, &mut symbols, errors);
    }
}

fn check_variable_declaration(
    decl: &VariableDeclaration,
    symbols: &mut Symbols,
    errors: &mut Vec<SemanticError>,
) {
    symbols.insert(decl.name.clone(), Type::Int);

    if let Some(init) = &decl.init {
        check_expression(init, symbols, errors);
    }
}

fn check_function_declaration(
    decl: &FunctionDeclaration,
    symbols: &mut Symbols,
    errors: &mut Vec<SemanticError>,
) {
    let has_body = decl.body.is_some();
    let mut new = FunctionType {
        num_parameters: decl.params.len(),
        defined: has_body,
    };

    if let Some(old_decl) = symbols.get(&decl.name) {
        match old_decl {
            Type::Function(old) => {
                new.defined = old.defined || has_body;

                // check if it was previously declared with a different type
                if old.num_parameters != new.num_parameters {
                    errors.push(SemanticError::IncompatibleFunctionDeclarations {
                        span: decl.span.clone(),
                        type_a: Type::Function(old.clone()),
                        type_b: Type::Function(new.clone()),
                    });
                }

                // check if defining a function that has already been defined
                if old.defined && has_body {
                    errors.push(SemanticError::FunctionRedefinition {
                        name: decl.name.1.clone(),
                        span: decl.span.clone(),
                    });
                }
            }
            _ => panic!("i hope this is unreachable"),
        }
    }

    symbols.insert(decl.name.clone(), Type::Function(new));

    if let Some(body) = &decl.body {
        for param in &decl.params {
            symbols.insert(param.name.clone(), Type::Int);
        }

        check_block(&body, symbols, errors);
    }
}

fn check_expression_inner(
    exp: &Expression,
    symbols: &mut Symbols,
    errors: &mut Vec<SemanticError>,
) {
    match &exp.kind {
        ExprKind::FunctionCall { name, args } => {
            if let Some(ty) = symbols.get(&name) {
                match ty {
                    Type::Function(FunctionType { num_parameters, .. }) => {
                        if *num_parameters != args.len() {
                            errors.push(SemanticError::WrongNumberOfArguements {
                                span: exp.span.clone(),
                                expected: *num_parameters,
                                found: args.len(),
                            });
                        }
                    }
                    _ => {
                        errors.push(SemanticError::VariableUsedAsFunction {
                            span: exp.span.clone(),
                            name: name.1.clone(),
                        });
                    }
                }
            } else {
                // has already been caught, we just keep going looking for other errors
            }
        }
        ExprKind::Var(v) => {
            if let Some(ty) = symbols.get(&v) {
                match ty {
                    Type::Function(_) => {
                        errors.push(SemanticError::FunctionUsedAsVariable {
                            name: v.1.clone(),
                            span: exp.span.clone(),
                        });
                    }
                    _ => {}
                }
            } else {
                // errors.push(SemanticError::UndeclaredVariable {
                //     name: v.1.clone(),
                //     span: exp.span.clone(),
                // });
            }
        }
        _ => (),
    }
}

fn check_expression(exp: &Expression, symbols: &mut Symbols, errors: &mut Vec<SemanticError>) {
    let mut state = (symbols, errors);
    exp.process_inner_expressions(&mut state, &|exp, (symbols, errors)| {
        check_expression_inner(exp, symbols, errors);
    });
}

fn check_block(block: &Vec<BlockItem>, symbols: &mut Symbols, errors: &mut Vec<SemanticError>) {
    for item in block {
        match item {
            BlockItem::Stmt(s) => check_statement(s, symbols, errors),
            BlockItem::Decl(decl) => match decl {
                Declaration::Var(v) => check_variable_declaration(v, symbols, errors),
                Declaration::Func(f) => check_function_declaration(f, symbols, errors),
            },
        }
    }
}

fn check_statement(stmt: &Statement, symbols: &mut Symbols, errors: &mut Vec<SemanticError>) {
    let mut state = (symbols, errors);
    stmt.process_inner_expressions(&mut state, &|exp, (symbols, errors)| {
        check_expression_inner(exp, symbols, errors);
    });
}
