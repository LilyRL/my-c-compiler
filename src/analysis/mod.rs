use std::collections::HashMap;

use crate::parser::{
    Block, BlockItem, Constant, Declaration, Expression, Identifier, Program, Statement,
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

type VariableMap = HashMap<Identifier, ScopedVariable>;

#[derive(Clone)]
struct ScopedVariable {
    resolved_name: Identifier,
    from_this_scope: bool,
}

fn create_inner_scope(outer_map: &VariableMap) -> VariableMap {
    let mut map = outer_map.clone();

    for (_, v) in map.iter_mut() {
        v.from_this_scope = false;
    }

    map
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

    resolve_block(&mut program.0.block, &mut variable_map)
}

fn resolve_block_item(
    item: &mut crate::parser::BlockItem,
    variable_map: &mut VariableMap,
) -> Result<(), SemanticError> {
    match item {
        crate::parser::BlockItem::Decl(decl) => resolve_declaration(decl, variable_map),
        crate::parser::BlockItem::Stmt(stmt) => resolve_statement(stmt, variable_map),
    }
}

fn resolve_block(block: &mut Block, variable_map: &mut VariableMap) -> Result<(), SemanticError> {
    for block_item in block.iter_mut() {
        resolve_block_item(block_item, variable_map)?;
    }

    Ok(())
}

fn resolve_statement(
    stmt: &mut Statement,
    variable_map: &mut VariableMap,
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
        Statement::Compound(block) => {
            let mut new_map = create_inner_scope(variable_map);
            resolve_block(block, &mut new_map)
        }
        Statement::Null | Statement::Goto(_) | Statement::Label(_) => Ok(()),
    }
}

fn resolve_declaration(
    Declaration { name, init }: &mut Declaration,
    variable_map: &mut VariableMap,
) -> Result<(), SemanticError> {
    if let Some(var) = variable_map.get(name)
        && var.from_this_scope
    {
        return Err(SemanticError::VariableRedeclaration(name.0.to_string()));
    }

    let unique_name = Identifier::new(&name.0);
    variable_map.insert(
        name.clone(),
        ScopedVariable {
            resolved_name: unique_name.clone(),
            from_this_scope: true,
        },
    );

    if let Some(init) = init {
        resolve_expression(init, variable_map)?;
    }

    *name = unique_name;

    Ok(())
}

fn resolve_expression(
    expr: &mut Expression,
    variable_map: &mut VariableMap,
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
                *i = unique_name.resolved_name.clone();
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
