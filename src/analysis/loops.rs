use crate::{
    analysis::errors::SemanticError,
    parser::{BlockItem, Identifier, Program, Statement, StmtKind},
};

pub fn label_all_loops(program: &mut Program, errors: &mut Vec<SemanticError>) {
    for function in &mut program.0 {
        if let Some(block) = &mut function.body {
            for block_item in block.iter_mut() {
                if let BlockItem::Stmt(stmt) = block_item {
                    loop_labeling(stmt, None, None, errors);
                }
            }
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
