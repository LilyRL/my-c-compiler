use std::collections::HashSet;

use crate::{
    analysis::errors::SemanticError,
    parser::{BlockItem, Constant, Identifier, Program, Statement, StmtKind, SwitchCase},
};

#[derive(Debug)]
struct SwitchCaseData<'a> {
    cases: &'a mut Vec<SwitchCase>,
    case_set: &'a mut HashSet<Constant>,
    default_case: &'a mut Option<Identifier>,
}

pub fn collect_all_switch_cases(program: &mut Program, errors: &mut Vec<SemanticError>) {
    for function in &mut program.0 {
        if let Some(block) = &mut function.body {
            for block_item in block.iter_mut() {
                if let BlockItem::Stmt(stmt) = block_item {
                    find_and_collect_switch_cases(stmt, errors);
                }
            }
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
