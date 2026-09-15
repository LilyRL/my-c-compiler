use std::collections::HashMap;

use strum::EnumIs;

use crate::{
    diagnostics::{Diagnostics, Span},
    parser::{
        Block, BlockItem, Declaration, ExprKind, Expression, ForInit, FunctionDeclaration,
        FunctionParameter, Identifier, Program, Statement, StmtKind, VariableDeclaration,
    },
};

type IdentifierMap = HashMap<Identifier, ScopedIdentifier>;

sge_global::global!(IdentifierMap, identifiers);

type Type = Identifier;

#[derive(EnumIs, Clone, Copy)]
pub enum Scope {
    Global,
    Local,
}

#[derive(Clone, Debug)]
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

pub fn resolve_all_identifiers(program: &mut Program, diagnostics: &mut Diagnostics) {
    let mut identifier_map = HashMap::new();

    for declaration in &mut program.0 {
        resolve_declaration(declaration, &mut identifier_map, diagnostics, Scope::Global);
    }

    set_identifiers(identifier_map);
}

fn resolve_block_item(
    item: &mut BlockItem,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    match item {
        BlockItem::Decl(decl) => {
            resolve_declaration(decl, identifier_map, diagnostics, Scope::Local)
        }
        BlockItem::Stmt(stmt) => resolve_statement(stmt, identifier_map, diagnostics),
    }
}

fn resolve_declaration(
    decl: &mut Declaration,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
    scope: Scope,
) {
    match decl {
        Declaration::Var(v) => resolve_variable_declaration(v, identifier_map, diagnostics, scope),
        Declaration::Func(f) => resolve_function_declaration(f, identifier_map, diagnostics, scope),
    }
}

fn resolve_block(
    block: &mut Block,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    for block_item in block.iter_mut() {
        resolve_block_item(block_item, identifier_map, diagnostics);
    }
}

fn resolve_statement(
    stmt: &mut Statement,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    match &mut stmt.kind {
        StmtKind::Return(e) | StmtKind::Expression(e) => {
            resolve_expression(e, identifier_map, diagnostics)
        }
        StmtKind::If { cond, then, else_ } => {
            resolve_expression(cond, identifier_map, diagnostics);
            resolve_statement(then, identifier_map, diagnostics);
            if let Some(else_stmt) = else_ {
                resolve_statement(else_stmt, identifier_map, diagnostics);
            }
        }
        StmtKind::Compound(block) => {
            let mut new_map = create_inner_scope(identifier_map);
            resolve_block(block, &mut new_map, diagnostics)
        }
        StmtKind::While { cond, body, .. } | StmtKind::DoWhile { body, cond, .. } => {
            resolve_expression(cond, identifier_map, diagnostics);
            resolve_statement(body, identifier_map, diagnostics);
        }
        StmtKind::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            let mut inner_scope = create_inner_scope(identifier_map);
            resolve_for_init(init, &mut inner_scope, diagnostics);
            resolve_optional_expression(condition, &mut inner_scope, diagnostics);
            resolve_optional_expression(post, &mut inner_scope, diagnostics);
            resolve_statement(body, &mut inner_scope, diagnostics);
        }
        StmtKind::Label(_, s) => resolve_statement(s, identifier_map, diagnostics),
        StmtKind::Null | StmtKind::Goto(_) | StmtKind::Continue(_) | StmtKind::Break(_) => {}
        StmtKind::Switch(s) => {
            resolve_expression(&mut s.value, identifier_map, diagnostics);
            resolve_statement(&mut s.body, identifier_map, diagnostics)
        }
        StmtKind::Case { stmt, .. } | StmtKind::DefaultCase { stmt, .. } => {
            resolve_statement(stmt, identifier_map, diagnostics)
        }
    }
}

fn resolve_function_declaration(
    decl: &mut FunctionDeclaration,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
    scope: Scope,
) {
    let FunctionDeclaration {
        name,
        params,
        body: block,
        name_span,
        storage_class,
        ..
    } = decl;

    if scope.is_local() && storage_class.is_static() {
        diagnostics.analysis_error(
            name_span.clone(),
            format!("function '{}' declared 'static' in a local scope", name.0),
        );
    }

    let mut defined = block.is_some();

    if let Some(prev_entry) = identifier_map.get(name) {
        defined = defined || prev_entry.defined;

        if prev_entry.from_this_scope && !prev_entry.external_linkage {
            diagnostics.analysis_error(
                name_span.clone(),
                format!("redeclaration of function '{}'", name.0),
            );
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
            resolve_parameter(param, &mut inner_map, diagnostics);
        }

        if let Some(block) = block {
            resolve_block(block, &mut inner_map, diagnostics);
        }
    }
}

fn resolve_var_like_identifier(
    name: &mut Identifier,
    init: Option<&mut Expression>,
    span: &Span,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    if let Some(var) = identifier_map.get(name)
        && var.from_this_scope
    {
        diagnostics.analysis_error(
            span.clone(),
            format!("redeclaration of variable '{}'", name.0),
        );
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
        resolve_expression(init, identifier_map, diagnostics);
    }

    *name = unique_name;
}

fn resolve_parameter(
    p: &mut FunctionParameter,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    resolve_var_like_identifier(&mut p.name, None, &p.span, identifier_map, diagnostics);
}

fn resolve_variable_declaration(
    decl: &mut VariableDeclaration,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
    scope: Scope,
) {
    if scope.is_global() {
        identifier_map.insert(
            decl.name.clone(),
            ScopedIdentifier {
                resolved_name: decl.name.clone(),
                from_this_scope: true,
                external_linkage: true,
                defined: true,
            },
        );
    } else if scope.is_local() {
        if let Some(prev_entry) = identifier_map.get(&decl.name)
            && prev_entry.from_this_scope
        {
            let valid_previous_entry =
                prev_entry.external_linkage && decl.storage_class.is_extern();
            if !valid_previous_entry {
                diagnostics.analysis_error(
                    decl.span.clone(),
                    format!(
                        "conflicting declarations for local variable '{}'",
                        decl.name.0
                    ),
                );
            }
        }

        if decl.storage_class.is_extern() {
            identifier_map.insert(
                decl.name.clone(),
                ScopedIdentifier {
                    resolved_name: decl.name.clone(),
                    from_this_scope: true,
                    external_linkage: true,
                    defined: true,
                },
            );
        } else {
            let new_name = Identifier::new(&decl.name);
            identifier_map.insert(
                decl.name.clone(),
                ScopedIdentifier {
                    resolved_name: new_name.clone(),
                    from_this_scope: true,
                    external_linkage: false,
                    defined: true,
                },
            );
            decl.name = new_name;
        }
    }

    resolve_optional_expression(&mut decl.init, identifier_map, diagnostics);
}

fn resolve_expression(
    expr: &mut Expression,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    match &mut expr.kind {
        ExprKind::Assignment(lhs, rhs) => {
            check_lvalue(lhs, diagnostics);
            resolve_expression(lhs, identifier_map, diagnostics);
            resolve_expression(rhs, identifier_map, diagnostics);
        }
        ExprKind::Var(i) => {
            if let Some(unique_name) = identifier_map.get(i) {
                let resolved = unique_name.resolved_name.clone();
                *i = resolved;
            } else {
                diagnostics
                    .analysis_error(expr.span.clone(), format!("undeclared variable '{}'", i.0));
            }
        }
        ExprKind::Unary { expr, .. } => resolve_expression(expr, identifier_map, diagnostics),
        ExprKind::Binary { lhs, rhs, .. } => {
            resolve_expression(lhs, identifier_map, diagnostics);
            resolve_expression(rhs, identifier_map, diagnostics)
        }
        ExprKind::CompoundAssign { lhs, rhs, .. } => {
            check_lvalue(lhs, diagnostics);
            resolve_expression(lhs, identifier_map, diagnostics);
            resolve_expression(rhs, identifier_map, diagnostics);
        }
        ExprKind::Postfix(_, expr) => {
            check_lvalue(expr, diagnostics);
            resolve_expression(expr, identifier_map, diagnostics);
        }
        ExprKind::Prefix(_, expr) => {
            check_lvalue(expr, diagnostics);
            resolve_expression(expr, identifier_map, diagnostics);
        }
        ExprKind::Conditional(a, b, c) => {
            resolve_expression(a, identifier_map, diagnostics);
            resolve_expression(b, identifier_map, diagnostics);
            resolve_expression(c, identifier_map, diagnostics)
        }
        ExprKind::FunctionCall { args, name } => {
            if let Some(i) = identifier_map.get(name) {
                *name = i.resolved_name.clone();
            } else {
                diagnostics.analysis_error(
                    expr.span.clone(),
                    format!("undeclared function '{}'", name.0),
                );
            }

            for arg in args {
                resolve_expression(arg, identifier_map, diagnostics);
            }
        }
        ExprKind::Constant(_) => {}
    }
}

fn check_lvalue(expr: &Expression, diagnostics: &mut Diagnostics) {
    if !expr.is_var() {
        diagnostics.analysis_error(
            expr.span.clone(),
            format!("invalid assignment target '{}'", expr.to_string()),
        );
    }
}

fn resolve_optional_expression(
    expr: &mut Option<Expression>,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    if let Some(e) = expr {
        resolve_expression(e, identifier_map, diagnostics);
    }
}

fn resolve_for_init(
    init: &mut ForInit,
    identifier_map: &mut IdentifierMap,
    diagnostics: &mut Diagnostics,
) {
    match init {
        ForInit::Expr(e) => resolve_expression(e, identifier_map, diagnostics),
        ForInit::Decl(decl) => {
            resolve_variable_declaration(decl, identifier_map, diagnostics, Scope::Local)
        }
        ForInit::None => {}
    }
}
