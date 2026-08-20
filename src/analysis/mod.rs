use std::collections::HashMap;

use crate::parser::{
    BlockItem, Constant, Declaration, Expression, Identifier, Program, Statement,
};

#[derive(Debug)]
pub enum SemanticError {
    InvalidLValue(String),
    VariableRedeclaration(String),
    UndeclaredVariable(String),
}

pub fn validate_program(program: &mut Program) -> Result<(), SemanticError> {
    add_return_zero(program);
    resolve_all_variables(program)
}

// TODO: when we add multiple functions, this only has to be applied to the main function
fn add_return_zero(program: &mut Program) {
    if let Some(last_item) = program.0.block.last() {
        if let crate::parser::BlockItem::Stmt(crate::parser::Statement::Return(_)) = last_item {
            return;
        }
    }

    program
        .0
        .block
        .push(BlockItem::Stmt(Statement::Return(Expression::Constant(
            Constant::Int(0),
        ))));
}

fn resolve_all_variables(program: &mut Program) -> Result<(), SemanticError> {
    let mut variable_map = HashMap::new();

    for block_item in program.0.block.iter_mut() {
        resolve_block_item(block_item, &mut variable_map)?;
    }

    Ok(())
}

fn resolve_block_item(
    item: &mut crate::parser::BlockItem,
    variable_map: &mut HashMap<Identifier, Identifier>,
) -> Result<(), SemanticError> {
    match item {
        crate::parser::BlockItem::Decl(decl) => resolve_declaration(decl, variable_map),
        crate::parser::BlockItem::Stmt(stmt) => resolve_statement(stmt, variable_map),
    }
}

fn resolve_statement(
    stmt: &mut Statement,
    variable_map: &mut HashMap<Identifier, Identifier>,
) -> Result<(), SemanticError> {
    match stmt {
        Statement::Return(e) => resolve_expression(e, variable_map),
        Statement::Expression(e) => resolve_expression(e, variable_map),
        Statement::If { cond, then, else_ } => {
            resolve_expression(cond, variable_map)?;
            resolve_statement(then, variable_map)?;
            if let Some(else_stmt) = else_ {
                resolve_statement(else_stmt, variable_map)?;
            }
            Ok(())
        }
        Statement::Null => Ok(()),
    }
}

fn resolve_declaration(
    Declaration { name, init }: &mut Declaration,
    variable_map: &mut HashMap<Identifier, Identifier>,
) -> Result<(), SemanticError> {
    if variable_map.contains_key(&name) {
        return Err(SemanticError::VariableRedeclaration(name.0.to_string()));
    }

    let unique_name = Identifier::new(&name.0);
    variable_map.insert(name.clone(), unique_name.clone());

    if let Some(init) = init {
        resolve_expression(init, variable_map)?;
    }

    *name = unique_name;

    Ok(())
}

fn resolve_expression(
    expr: &mut Expression,
    variable_map: &mut HashMap<Identifier, Identifier>,
) -> Result<(), SemanticError> {
    match expr {
        Expression::Assignment(l, r) => {
            if !l.is_var() {
                return Err(SemanticError::InvalidLValue(l.to_string()));
            }

            resolve_expression(l, variable_map)?;
            resolve_expression(r, variable_map)?;

            Ok(())
        }
        Expression::Var(i) => {
            if let Some(unique_name) = variable_map.get(i) {
                *i = unique_name.clone();
                Ok(())
            } else {
                Err(SemanticError::UndeclaredVariable(i.0.to_string()))
            }
        }
        Expression::Unary { expr, .. } => resolve_expression(expr, variable_map),
        Expression::Binary { lhs, rhs, .. } => {
            resolve_expression(lhs, variable_map)?;
            resolve_expression(rhs, variable_map)
        }
        Expression::CompoundAssign { lhs, rhs, .. } => {
            if !lhs.is_var() {
                return Err(SemanticError::InvalidLValue(lhs.to_string()));
            }

            resolve_expression(lhs, variable_map)?;
            resolve_expression(rhs, variable_map)?;

            Ok(())
        }
        Expression::Postfix(_, expr) => {
            if !expr.is_var() {
                return Err(SemanticError::InvalidLValue(expr.to_string()));
            }

            resolve_expression(expr, variable_map)?;
            Ok(())
        }
        Expression::Prefix(_, expr) => {
            if !expr.is_var() {
                return Err(SemanticError::InvalidLValue(expr.to_string()));
            }

            resolve_expression(expr, variable_map)?;
            Ok(())
        }
        Expression::Conditional(a, b, c) => {
            resolve_expression(a, variable_map)?;
            resolve_expression(b, variable_map)?;
            resolve_expression(c, variable_map)
        }
        Expression::Constant(_) => Ok(()),
    }
}
