use std::collections::HashMap;

use crate::{
    analysis::errors::SemanticError,
    diagnostics::Span,
    parser::{
        Block, BlockItem, Declaration, ExprKind, Expression, ForInit, FunctionDeclaration,
        FunctionParameter, Identifier, Program, Statement, StmtKind, VariableDeclaration,
    },
};

type IdentifierMap = HashMap<Identifier, ScopedIdentifier>;

sge_global::global!(IdentifierMap, identifiers);

type Type = Identifier;

#[derive(Clone)]
pub struct ScopedIdentifier {
    pub resolved_name: Type,
    pub from_this_scope: bool,
    pub external_linkage: bool,
    pub defined: bool,
}

fn create_inner_scope(outer_map: &IdentifierMap) -> IdentifierMap {
    let mut map = outer_map.clone();

    for (_, v) in map.iter_mut() {
        v.from_this_scope = false;
    }

    map
}

pub fn resolve_all_identifiers(program: &mut Program, errors: &mut Vec<SemanticError>) {
    let mut identifier_map = HashMap::new();

    for function in &mut program.0 {
        resolve_function_declaration(function, &mut identifier_map, errors);
    }

    set_identifiers(identifier_map);
}

fn resolve_block_item(
    item: &mut BlockItem,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    match item {
        BlockItem::Decl(decl) => resolve_declaration(decl, identifier_map, errors),
        BlockItem::Stmt(stmt) => resolve_statement(stmt, identifier_map, errors),
    }
}

fn resolve_declaration(
    decl: &mut Declaration,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    match decl {
        Declaration::Var(v) => resolve_variable_declaration(v, identifier_map, errors),
        Declaration::Func(f) => resolve_function_declaration(f, identifier_map, errors),
    }
}

fn resolve_block(
    block: &mut Block,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    for block_item in block.iter_mut() {
        resolve_block_item(block_item, identifier_map, errors);
    }
}

fn resolve_statement(
    stmt: &mut Statement,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    match &mut stmt.kind {
        StmtKind::Return(e) | StmtKind::Expression(e) => {
            resolve_expression(e, identifier_map, errors)
        }
        StmtKind::If { cond, then, else_ } => {
            resolve_expression(cond, identifier_map, errors);
            resolve_statement(then, identifier_map, errors);
            if let Some(else_stmt) = else_ {
                resolve_statement(else_stmt, identifier_map, errors);
            }
        }
        StmtKind::Compound(block) => {
            let mut new_map = create_inner_scope(identifier_map);
            resolve_block(block, &mut new_map, errors)
        }
        StmtKind::While { cond, body, .. } | StmtKind::DoWhile { body, cond, .. } => {
            resolve_expression(cond, identifier_map, errors);
            resolve_statement(body, identifier_map, errors);
        }
        StmtKind::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            let mut inner_scope = create_inner_scope(identifier_map);
            resolve_for_init(init, &mut inner_scope, errors);
            resolve_optional_expression(condition, &mut inner_scope, errors);
            resolve_optional_expression(post, &mut inner_scope, errors);
            resolve_statement(body, &mut inner_scope, errors);
        }
        StmtKind::Label(_, s) => resolve_statement(s, identifier_map, errors),
        StmtKind::Null | StmtKind::Goto(_) | StmtKind::Continue(_) | StmtKind::Break(_) => {}
        StmtKind::Switch(s) => {
            resolve_expression(&mut s.value, identifier_map, errors);
            resolve_statement(&mut s.body, identifier_map, errors)
        }
        StmtKind::Case { stmt, .. } | StmtKind::DefaultCase { stmt, .. } => {
            resolve_statement(stmt, identifier_map, errors)
        }
    }
}

fn resolve_function_declaration(
    decl: &mut FunctionDeclaration,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    let FunctionDeclaration {
        name,
        params,
        body: block,
        name_span,
        ..
    } = decl;

    let mut defined = block.is_some();

    if let Some(prev_entry) = identifier_map.get(name) {
        defined = defined || prev_entry.defined;

        if prev_entry.from_this_scope && !prev_entry.external_linkage {
            errors.push(SemanticError::FunctionRedeclaration {
                name: name.0.clone(),
                span: name_span.clone(),
            });

            return;
        }
    }

    identifier_map.insert(
        name.clone(),
        ScopedIdentifier {
            resolved_name: name.clone(),
            from_this_scope: true,
            external_linkage: true,
            defined,
        },
    );

    let something_to_resolve = !(params.is_empty() && block.is_none());
    if something_to_resolve {
        let mut inner_map = create_inner_scope(identifier_map);

        for param in params {
            resolve_parameter(param, &mut inner_map, errors);
        }

        if let Some(block) = block {
            resolve_block(block, &mut inner_map, errors);
        }
    }
}

fn resolve_var_like_identifier(
    name: &mut Identifier,
    init: Option<&mut Expression>,
    span: &Span,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    if let Some(var) = identifier_map.get(name)
        && var.from_this_scope
    {
        errors.push(SemanticError::VariableRedeclaration {
            name: name.0.clone(),
            span: span.clone(),
        });
    }

    let unique_name = Identifier::new(&name.0);
    identifier_map.insert(
        name.clone(),
        ScopedIdentifier {
            resolved_name: unique_name.clone(),
            from_this_scope: true,
            external_linkage: false,
            defined: true,
        },
    );

    if let Some(init) = init {
        resolve_expression(init, identifier_map, errors);
    }

    *name = unique_name;
}

fn resolve_parameter(
    p: &mut FunctionParameter,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    resolve_var_like_identifier(&mut p.name, None, &p.span, identifier_map, errors);
}

fn resolve_variable_declaration(
    decl: &mut VariableDeclaration,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    let VariableDeclaration { name, init, span } = decl;
    resolve_var_like_identifier(name, init.as_mut(), span, identifier_map, errors);
}

fn resolve_expression(
    expr: &mut Expression,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    match &mut expr.kind {
        ExprKind::Assignment(lhs, rhs) => {
            check_lvalue(lhs, errors);
            resolve_expression(lhs, identifier_map, errors);
            resolve_expression(rhs, identifier_map, errors);
        }
        ExprKind::Var(i) => {
            if let Some(unique_name) = identifier_map.get(i) {
                let resolved = unique_name.resolved_name.clone();
                *i = resolved;
            } else {
                errors.push(SemanticError::UndeclaredVariable {
                    name: i.0.clone(),
                    span: expr.span.clone(),
                });
            }
        }
        ExprKind::Unary { expr, .. } => resolve_expression(expr, identifier_map, errors),
        ExprKind::Binary { lhs, rhs, .. } => {
            resolve_expression(lhs, identifier_map, errors);
            resolve_expression(rhs, identifier_map, errors)
        }
        ExprKind::CompoundAssign { lhs, rhs, .. } => {
            check_lvalue(lhs, errors);
            resolve_expression(lhs, identifier_map, errors);
            resolve_expression(rhs, identifier_map, errors);
        }
        ExprKind::Postfix(_, expr) => {
            check_lvalue(expr, errors);
            resolve_expression(expr, identifier_map, errors);
        }
        ExprKind::Prefix(_, expr) => {
            check_lvalue(expr, errors);
            resolve_expression(expr, identifier_map, errors);
        }
        ExprKind::Conditional(a, b, c) => {
            resolve_expression(a, identifier_map, errors);
            resolve_expression(b, identifier_map, errors);
            resolve_expression(c, identifier_map, errors)
        }
        ExprKind::FunctionCall { args, name } => {
            if let Some(i) = identifier_map.get(name) {
                *name = i.resolved_name.clone();
            } else {
                errors.push(SemanticError::UndeclaredFunction {
                    name: name.0.clone(),
                    span: expr.span.clone(),
                });
            }

            for arg in args {
                resolve_expression(arg, identifier_map, errors);
            }
        }
        ExprKind::Constant(_) => {}
    }
}

fn check_lvalue(expr: &Expression, errors: &mut Vec<SemanticError>) {
    if !expr.is_var() {
        errors.push(SemanticError::InvalidLValue {
            expr: expr.to_string(),
            span: expr.span.clone(),
        });
    }
}

fn resolve_optional_expression(
    expr: &mut Option<Expression>,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    if let Some(e) = expr {
        resolve_expression(e, identifier_map, errors);
    }
}

fn resolve_for_init(
    init: &mut ForInit,
    identifier_map: &mut IdentifierMap,
    errors: &mut Vec<SemanticError>,
) {
    match init {
        ForInit::Expr(e) => resolve_expression(e, identifier_map, errors),
        ForInit::Decl(decl) => resolve_variable_declaration(decl, identifier_map, errors),
        ForInit::None => {}
    }
}
