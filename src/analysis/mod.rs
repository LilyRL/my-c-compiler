use std::collections::{HashMap, HashSet};

use crate::diagnostics::Span;
use crate::parser::{
    Block, BlockItem, Constant, Declaration, ExprKind, Expression, ForInit, Identifier, Program,
    Statement, StmtKind, SwitchCase,
};

#[derive(Debug)]
pub enum SemanticError {
    InvalidLValue { expr: String, span: Span },
    VariableRedeclaration { name: String, span: Span },
    UndeclaredVariable { name: String, span: Span },
    BreakOutsideBreakable { span: Span },
    ContinueOutsideLoop { span: Span },
    DuplicateSwitchCase { value: Constant, span: Span },
    DuplicateDefaultSwitchCase { span: Span },
    CaseOutsideSwitch { span: Span },
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
            | Self::CaseOutsideSwitch { span } => span,
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
        }
    }
}

pub fn validate_program(program: &mut Program) -> Vec<SemanticError> {
    add_return_zero(program);

    let mut errors = Vec::new();
    resolve_all_variables(program, &mut errors);
    label_all_loops(program, &mut errors);
    collect_all_switch_cases(program, &mut errors);

    errors
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
        if let BlockItem::Stmt(Statement {
            kind: StmtKind::Return(_),
            ..
        }) = last_item
        {
            return;
        }
    }

    program.0.block.push(BlockItem::Stmt(Statement::new(
        StmtKind::Return(Expression::new(ExprKind::Constant(Constant::Int(0)), 0..0)),
        0..0,
    )));
}

fn resolve_all_variables(program: &mut Program, errors: &mut Vec<SemanticError>) {
    let mut variable_map = HashMap::new();

    resolve_block(&mut program.0.block, &mut variable_map, errors)
}

fn resolve_block_item(
    item: &mut BlockItem,
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    match item {
        BlockItem::Decl(decl) => resolve_declaration(decl, variable_map, errors),
        BlockItem::Stmt(stmt) => resolve_statement(stmt, variable_map, errors),
    }
}

fn resolve_block(
    block: &mut Block,
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    for block_item in block.iter_mut() {
        resolve_block_item(block_item, variable_map, errors);
    }
}

fn resolve_statement(
    stmt: &mut Statement,
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    match &mut stmt.kind {
        StmtKind::Return(e) | StmtKind::Expression(e) => {
            resolve_expression(e, variable_map, errors)
        }
        StmtKind::If { cond, then, else_ } => {
            resolve_expression(cond, variable_map, errors);
            resolve_statement(then, variable_map, errors);
            if let Some(else_stmt) = else_ {
                resolve_statement(else_stmt, variable_map, errors);
            }
        }
        StmtKind::Compound(block) => {
            let mut new_map = create_inner_scope(variable_map);
            resolve_block(block, &mut new_map, errors)
        }
        StmtKind::While { cond, body, .. } | StmtKind::DoWhile { body, cond, .. } => {
            resolve_expression(cond, variable_map, errors);
            resolve_statement(body, variable_map, errors);
        }
        StmtKind::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            let mut inner_scope = create_inner_scope(variable_map);
            resolve_for_init(init, &mut inner_scope, errors);
            resolve_optional_expression(condition, &mut inner_scope, errors);
            resolve_optional_expression(post, &mut inner_scope, errors);
            resolve_statement(body, &mut inner_scope, errors);
        }
        StmtKind::Label(_, s) => resolve_statement(s, variable_map, errors),
        StmtKind::Null | StmtKind::Goto(_) | StmtKind::Continue(_) | StmtKind::Break(_) => {}
        StmtKind::Switch(s) => {
            resolve_expression(&mut s.value, variable_map, errors);
            resolve_statement(&mut s.body, variable_map, errors)
        }
        StmtKind::Case { stmt, .. } | StmtKind::DefaultCase { stmt, .. } => {
            resolve_statement(stmt, variable_map, errors)
        }
    }
}

fn resolve_declaration(
    decl: &mut Declaration,
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    let Declaration { name, init, span } = decl;

    if let Some(var) = variable_map.get(name)
        && var.from_this_scope
    {
        errors.push(SemanticError::VariableRedeclaration {
            name: name.0.clone(),
            span: span.clone(),
        });
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
        resolve_expression(init, variable_map, errors);
    }

    *name = unique_name;
}

fn resolve_expression(
    expr: &mut Expression,
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    match &mut expr.kind {
        ExprKind::Assignment(lhs, rhs) => {
            check_lvalue(lhs, errors);
            resolve_expression(lhs, variable_map, errors);
            resolve_expression(rhs, variable_map, errors);
        }
        ExprKind::Var(i) => {
            if let Some(unique_name) = variable_map.get(i) {
                let resolved = unique_name.resolved_name.clone();
                *i = resolved;
            } else {
                errors.push(SemanticError::UndeclaredVariable {
                    name: i.0.clone(),
                    span: expr.span.clone(),
                });
            }
        }
        ExprKind::Unary { expr, .. } => resolve_expression(expr, variable_map, errors),
        ExprKind::Binary { lhs, rhs, .. } => {
            resolve_expression(lhs, variable_map, errors);
            resolve_expression(rhs, variable_map, errors)
        }
        ExprKind::CompoundAssign { lhs, rhs, .. } => {
            check_lvalue(lhs, errors);
            resolve_expression(lhs, variable_map, errors);
            resolve_expression(rhs, variable_map, errors);
        }
        ExprKind::Postfix(_, expr) => {
            check_lvalue(expr, errors);
            resolve_expression(expr, variable_map, errors);
        }
        ExprKind::Prefix(_, expr) => {
            check_lvalue(expr, errors);
            resolve_expression(expr, variable_map, errors);
        }
        ExprKind::Conditional(a, b, c) => {
            resolve_expression(a, variable_map, errors);
            resolve_expression(b, variable_map, errors);
            resolve_expression(c, variable_map, errors)
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
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    if let Some(e) = expr {
        resolve_expression(e, variable_map, errors);
    }
}

fn resolve_for_init(
    init: &mut ForInit,
    variable_map: &mut VariableMap,
    errors: &mut Vec<SemanticError>,
) {
    match init {
        ForInit::Expr(e) => resolve_expression(e, variable_map, errors),
        ForInit::Decl(decl) => resolve_declaration(decl, variable_map, errors),
        ForInit::None => {}
    }
}

fn label_all_loops(program: &mut Program, errors: &mut Vec<SemanticError>) {
    for block_item in program.0.block.iter_mut() {
        if let BlockItem::Stmt(stmt) = block_item {
            loop_labeling(stmt, None, None, errors);
        }
    }
}

fn loop_labeling(
    stmt: &mut Statement,
    closest_breakable: Option<&Identifier>,
    closest_continuable: Option<&Identifier>,
    errors: &mut Vec<SemanticError>,
) {
    match &mut stmt.kind {
        StmtKind::Break(i) => match closest_breakable {
            Some(loop_identifier) => *i = loop_identifier.clone(),
            None => errors.push(SemanticError::BreakOutsideBreakable {
                span: stmt.span.clone(),
            }),
        },
        StmtKind::Continue(i) => match closest_continuable {
            Some(loop_identifier) => *i = loop_identifier.clone(),
            None => errors.push(SemanticError::ContinueOutsideLoop {
                span: stmt.span.clone(),
            }),
        },
        StmtKind::If { then, else_, .. } => {
            loop_labeling(then, closest_breakable, closest_continuable, errors);
            if let Some(else_) = else_ {
                loop_labeling(else_, closest_breakable, closest_continuable, errors);
            }
        }
        StmtKind::While { body, label, .. }
        | StmtKind::DoWhile { body, label, .. }
        | StmtKind::For { body, label, .. } => {
            loop_labeling(body, Some(label), Some(label), errors);
        }
        StmtKind::Compound(items) => {
            for item in items {
                if let BlockItem::Stmt(stmt) = item {
                    loop_labeling(stmt, closest_breakable, closest_continuable, errors);
                }
            }
        }
        StmtKind::Label(_, s) => loop_labeling(s, closest_breakable, closest_continuable, errors),
        StmtKind::Switch(s) => {
            loop_labeling(&mut s.body, Some(&s.label), closest_continuable, errors);
        }
        StmtKind::Case { stmt, .. } | StmtKind::DefaultCase { stmt, .. } => {
            loop_labeling(stmt, closest_breakable, closest_continuable, errors);
        }
        _ => {}
    }
}

#[derive(Debug)]
struct SwitchCaseData<'a> {
    cases: &'a mut Vec<SwitchCase>,
    case_set: &'a mut HashSet<Constant>,
    default_case: &'a mut Option<Identifier>,
}

fn collect_all_switch_cases(program: &mut Program, errors: &mut Vec<SemanticError>) {
    for block_item in program.0.block.iter_mut() {
        if let BlockItem::Stmt(stmt) = block_item {
            find_and_collect_switch_cases(stmt, errors);
        }
    }
}

fn find_and_collect_switch_cases(stmt: &mut Statement, errors: &mut Vec<SemanticError>) {
    match &mut stmt.kind {
        StmtKind::Case { header_span, .. } | StmtKind::DefaultCase { header_span, .. } => errors
            .push(SemanticError::CaseOutsideSwitch {
                span: header_span.clone(),
            }),
        StmtKind::If { then, else_, .. } => {
            find_and_collect_switch_cases(then, errors);
            if let Some(else_) = else_ {
                find_and_collect_switch_cases(else_, errors);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Label(_, body) => {
            find_and_collect_switch_cases(body, errors);
        }
        StmtKind::Compound(items) => {
            for item in items {
                if let BlockItem::Stmt(stmt) = item {
                    find_and_collect_switch_cases(stmt, errors);
                }
            }
        }
        StmtKind::Switch(s) => {
            let mut data = SwitchCaseData {
                cases: &mut s.cases,
                case_set: &mut s.case_set,
                default_case: &mut s.default_case,
            };
            collect_switch_cases(&mut s.body, &mut data, errors);
        }
        _ => {}
    }
}

fn collect_switch_cases(
    stmt: &mut Statement,
    data: &mut SwitchCaseData<'_>,
    errors: &mut Vec<SemanticError>,
) {
    match &mut stmt.kind {
        StmtKind::If { then, else_, .. } => {
            collect_switch_cases(then, data, errors);
            if let Some(else_) = else_ {
                collect_switch_cases(else_, data, errors);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Label(_, body) => {
            collect_switch_cases(body, data, errors);
        }
        StmtKind::Compound(items) => {
            for item in items {
                if let BlockItem::Stmt(stmt) = item {
                    collect_switch_cases(stmt, data, errors);
                }
            }
        }
        StmtKind::Switch(s) => {
            let mut data = SwitchCaseData {
                cases: &mut s.cases,
                case_set: &mut s.case_set,
                default_case: &mut s.default_case,
            };
            collect_switch_cases(&mut s.body, &mut data, errors);
        }
        StmtKind::Case {
            value,
            label,
            header_span,
            stmt,
        } => {
            if data.case_set.contains(value) {
                errors.push(SemanticError::DuplicateSwitchCase {
                    value: value.clone(),
                    span: header_span.clone(),
                });
            }

            data.case_set.insert(value.clone());
            data.cases.push((label.clone(), value.clone()));
            collect_switch_cases(stmt, data, errors);
        }
        StmtKind::DefaultCase {
            label,
            header_span,
            stmt,
        } => {
            if data.default_case.is_some() {
                errors.push(SemanticError::DuplicateDefaultSwitchCase {
                    span: header_span.clone(),
                });
            } else {
                *data.default_case = Some(label.clone());
            }

            collect_switch_cases(stmt, data, errors);
        }
        _ => {}
    }
}
