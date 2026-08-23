use std::collections::{HashMap, HashSet};

use crate::parser::{
    Block, BlockItem, Constant, Declaration, Expression, ForInit, Identifier, Program, Statement,
    Switch, SwitchCase,
};

#[derive(Debug)]
pub enum SemanticError {
    InvalidLValue(String),
    VariableRedeclaration(String),
    UndeclaredVariable(String),
    BreakOutsideBreakable,
    ContinueOutsideLoop,
    DuplicateSwitchCase(Constant),
    DuplicateDefaultSwitchCase,
    CaseOutsideSwitch,
}

pub fn validate_program(program: &mut Program) -> Result<(), SemanticError> {
    add_return_zero(program);
    resolve_all_variables(program)?;
    label_all_loops(program)?;
    collect_all_switch_cases(program)?;

    Ok(())
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
        Statement::While { cond, body, .. } | Statement::DoWhile { body, cond, .. } => {
            resolve_expression(cond, variable_map)?;
            resolve_statement(body, variable_map)?;
            Ok(())
        }
        Statement::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            let mut inner_scope = create_inner_scope(variable_map);
            resolve_for_init(init, &mut inner_scope)?;
            resolve_optional_expression(condition, &mut inner_scope)?;
            resolve_optional_expression(post, &mut inner_scope)?;
            resolve_statement(body, &mut inner_scope)?;
            Ok(())
        }
        Statement::Label(_, s) => resolve_statement(s, variable_map),
        Statement::Null | Statement::Goto(_) | Statement::Continue(_) | Statement::Break(_) => {
            Ok(())
        }
        Statement::Switch(s) => {
            resolve_expression(&mut s.value, variable_map)?;
            resolve_statement(&mut s.body, variable_map)
        }
        Statement::Case { stmt, .. } | Statement::DefaultCase { stmt, .. } => {
            resolve_statement(stmt, variable_map)
        }
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

fn resolve_optional_expression(
    expr: &mut Option<Expression>,
    variable_map: &mut VariableMap,
) -> Result<(), SemanticError> {
    if let Some(e) = expr {
        resolve_expression(e, variable_map)?;
    }

    Ok(())
}

fn resolve_for_init(
    init: &mut ForInit,
    variable_map: &mut VariableMap,
) -> Result<(), SemanticError> {
    match init {
        ForInit::Expr(e) => resolve_expression(e, variable_map),
        ForInit::Decl(decl) => resolve_declaration(decl, variable_map),
        ForInit::None => Ok(()),
    }
}

fn label_all_loops(program: &mut Program) -> Result<(), SemanticError> {
    for block_item in program.0.block.iter_mut() {
        if let BlockItem::Stmt(stmt) = block_item {
            loop_labeling(stmt, None, None)?;
        }
    }

    Ok(())
}

fn loop_labeling(
    stmt: &mut Statement,
    closest_breakable: Option<&Identifier>,
    closest_continuable: Option<&Identifier>,
) -> Result<(), SemanticError> {
    match stmt {
        Statement::Break(i) => match closest_breakable {
            Some(loop_identifier) => *i = loop_identifier.clone(),
            None => return Err(SemanticError::BreakOutsideBreakable),
        },
        Statement::Continue(i) => match closest_continuable {
            Some(loop_identifier) => *i = loop_identifier.clone(),
            None => return Err(SemanticError::ContinueOutsideLoop),
        },
        Statement::If { then, else_, .. } => {
            loop_labeling(then, closest_breakable, closest_continuable)?;
            if let Some(else_) = else_ {
                loop_labeling(else_, closest_breakable, closest_continuable)?;
            }
        }
        Statement::While { body, label, .. }
        | Statement::DoWhile { body, label, .. }
        | Statement::For { body, label, .. } => {
            let closest_breakable = Some(&*label);
            let closest_continuable = Some(&*label);
            loop_labeling(body, closest_breakable, closest_continuable)?;
        }
        Statement::Compound(items) => {
            for item in items {
                match item {
                    BlockItem::Stmt(stmt) => {
                        loop_labeling(stmt, closest_breakable, closest_continuable)?
                    }
                    _ => (),
                };
            }
        }
        Statement::Label(_, s) => loop_labeling(s, closest_breakable, closest_continuable)?,
        Statement::Switch(s) => {
            let closest_breakable = Some(&s.label);
            loop_labeling(&mut s.body, closest_breakable, closest_continuable)?;
        }
        Statement::Case { stmt, .. } | Statement::DefaultCase { stmt, .. } => {
            loop_labeling(stmt, closest_breakable, closest_continuable)?;
        }
        _ => (),
    }

    Ok(())
}

#[derive(Debug)]
struct SwitchCaseData<'a> {
    cases: &'a mut Vec<SwitchCase>,
    case_set: &'a mut HashSet<Constant>,
    default_case: &'a mut Option<Identifier>,
}

fn collect_all_switch_cases(program: &mut Program) -> Result<(), SemanticError> {
    for block_item in program.0.block.iter_mut() {
        if let BlockItem::Stmt(stmt) = block_item {
            find_and_collect_switch_cases(stmt)?;
        }
    }

    Ok(())
}

fn find_and_collect_switch_cases(stmt: &mut Statement) -> Result<(), SemanticError> {
    match stmt {
        Statement::Case { .. } => return Err(SemanticError::CaseOutsideSwitch),
        Statement::DefaultCase { .. } => return Err(SemanticError::CaseOutsideSwitch),
        Statement::If { then, else_, .. } => {
            find_and_collect_switch_cases(then)?;
            if let Some(else_) = else_ {
                find_and_collect_switch_cases(else_)?;
            }
        }
        Statement::While { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::For { body, .. }
        | Statement::Label(_, body) => {
            find_and_collect_switch_cases(body)?;
        }
        Statement::Compound(items) => {
            for item in items {
                match item {
                    BlockItem::Stmt(stmt) => find_and_collect_switch_cases(stmt)?,
                    _ => (),
                };
            }
        }
        Statement::Switch(s) => {
            let mut data = SwitchCaseData {
                cases: &mut s.cases,
                case_set: &mut s.case_set,
                default_case: &mut s.default_case,
            };
            collect_switch_cases(&mut s.body, &mut data)?;
        }
        _ => (),
    }

    Ok(())
}

fn collect_switch_cases(
    stmt: &mut Statement,
    data: &mut SwitchCaseData<'_>,
) -> Result<(), SemanticError> {
    match stmt {
        Statement::If { then, else_, .. } => {
            collect_switch_cases(then, data)?;
            if let Some(else_) = else_ {
                collect_switch_cases(else_, data)?;
            }
        }
        Statement::While { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::For { body, .. }
        | Statement::Label(_, body) => {
            collect_switch_cases(body, data)?;
        }
        Statement::Compound(items) => {
            for item in items {
                match item {
                    BlockItem::Stmt(stmt) => collect_switch_cases(stmt, data)?,
                    _ => (),
                };
            }
        }
        Statement::Switch(s) => {
            let mut data = SwitchCaseData {
                cases: &mut s.cases,
                case_set: &mut s.case_set,
                default_case: &mut s.default_case,
            };
            collect_switch_cases(&mut s.body, &mut data)?;
        }
        Statement::Case { value, label, stmt } => {
            if data.case_set.contains(&value) {
                dbg!(data);
                dbg!(label);
                return Err(SemanticError::DuplicateSwitchCase(value.clone()));
            }

            data.case_set.insert(value.clone());
            data.cases.push((label.clone(), value.clone()));
            collect_switch_cases(stmt, data)?;
        }
        Statement::DefaultCase { label, stmt } => {
            if data.default_case.is_some() {
                return Err(SemanticError::DuplicateDefaultSwitchCase);
            }

            *data.default_case = Some(label.clone());
            collect_switch_cases(stmt, data)?;
        }
        _ => (),
    }

    Ok(())
}
