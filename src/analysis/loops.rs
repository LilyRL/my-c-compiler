use crate::{
    diagnostics::Diagnostics,
    parser::{BlockItem, Identifier, Program, Statement, StmtKind},
};

pub fn label_all_loops(program: &mut Program, diagnostics: &mut Diagnostics) {
    for function in program.functions_mut() {
        if let Some(block) = &mut function.body {
            for block_item in block.iter_mut() {
                if let BlockItem::Stmt(stmt) = block_item {
                    loop_labeling(stmt, None, None, diagnostics);
                }
            }
        }
    }
}

fn loop_labeling(
    stmt: &mut Statement,
    closest_breakable: Option<&Identifier>,
    closest_continuable: Option<&Identifier>,
    diagnostics: &mut Diagnostics,
) {
    match &mut stmt.kind {
        StmtKind::Break(i) => match closest_breakable {
            Some(loop_identifier) => *i = loop_identifier.clone(),
            None => {
                diagnostics.analysis_error(stmt.span.clone(), "'break' outside of a loop or switch")
            }
        },
        StmtKind::Continue(i) => match closest_continuable {
            Some(loop_identifier) => *i = loop_identifier.clone(),
            None => diagnostics.analysis_error(stmt.span.clone(), "'continue' outside of a loop"),
        },
        StmtKind::If { then, else_, .. } => {
            loop_labeling(then, closest_breakable, closest_continuable, diagnostics);
            if let Some(else_) = else_ {
                loop_labeling(else_, closest_breakable, closest_continuable, diagnostics);
            }
        }
        StmtKind::While { body, label, .. }
        | StmtKind::DoWhile { body, label, .. }
        | StmtKind::For { body, label, .. } => {
            loop_labeling(body, Some(label), Some(label), diagnostics);
        }
        StmtKind::Compound(items) => {
            for item in items {
                if let BlockItem::Stmt(stmt) = item {
                    loop_labeling(stmt, closest_breakable, closest_continuable, diagnostics);
                }
            }
        }
        StmtKind::Label(_, s) => {
            loop_labeling(s, closest_breakable, closest_continuable, diagnostics)
        }
        StmtKind::Switch(s) => {
            loop_labeling(
                &mut s.body,
                Some(&s.label),
                closest_continuable,
                diagnostics,
            );
        }
        StmtKind::Case { stmt, .. } | StmtKind::DefaultCase { stmt, .. } => {
            loop_labeling(stmt, closest_breakable, closest_continuable, diagnostics);
        }
        _ => {}
    }
}
