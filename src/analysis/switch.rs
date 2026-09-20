use std::collections::HashSet;

use crate::{
    diagnostics::Diagnostics,
    parser::{
        BlockItem, Constant, ConstantType, Identifier, Program, Statement, StmtKind, SwitchCase,
    },
};

#[derive(Debug)]
struct SwitchCaseData<'a> {
    value_ty: ConstantType,
    cases: &'a mut Vec<SwitchCase>,
    case_set: &'a mut HashSet<Constant>,
    default_case: &'a mut Option<Identifier>,
}

pub fn collect_all_switch_cases(program: &mut Program, diagnostics: &mut Diagnostics) {
    for function in program.functions_mut() {
        if let Some(block) = &mut function.body {
            for block_item in block.iter_mut() {
                if let BlockItem::Stmt(stmt) = block_item {
                    find_and_collect_switch_cases(stmt, diagnostics);
                }
            }
        }
    }
}

fn find_and_collect_switch_cases(stmt: &mut Statement, diagnostics: &mut Diagnostics) {
    match &mut stmt.kind {
        StmtKind::Case { header_span, .. } | StmtKind::DefaultCase { header_span, .. } => {
            diagnostics.analysis_error(
                header_span.clone(),
                "'case'/'default' label outside of a switch",
            )
        }
        StmtKind::If { then, else_, .. } => {
            find_and_collect_switch_cases(then, diagnostics);
            if let Some(else_) = else_ {
                find_and_collect_switch_cases(else_, diagnostics);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Label(_, body) => {
            find_and_collect_switch_cases(body, diagnostics);
        }
        StmtKind::Compound(items) => {
            for item in items {
                if let BlockItem::Stmt(stmt) = item {
                    find_and_collect_switch_cases(stmt, diagnostics);
                }
            }
        }
        StmtKind::Switch(s) => {
            let mut data = SwitchCaseData {
                cases: &mut s.cases,
                case_set: &mut s.case_set,
                default_case: &mut s.default_case,
                value_ty: s.value.ty.to_constant().unwrap(),
            };
            collect_switch_cases(&mut s.body, &mut data, diagnostics);
        }
        _ => {}
    }
}

fn collect_switch_cases(
    stmt: &mut Statement,
    data: &mut SwitchCaseData<'_>,
    diagnostics: &mut Diagnostics,
) {
    match &mut stmt.kind {
        StmtKind::If { then, else_, .. } => {
            collect_switch_cases(then, data, diagnostics);
            if let Some(else_) = else_ {
                collect_switch_cases(else_, data, diagnostics);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Label(_, body) => {
            collect_switch_cases(body, data, diagnostics);
        }
        StmtKind::Compound(items) => {
            for item in items {
                if let BlockItem::Stmt(stmt) = item {
                    collect_switch_cases(stmt, data, diagnostics);
                }
            }
        }
        StmtKind::Switch(s) => {
            let mut data = SwitchCaseData {
                cases: &mut s.cases,
                case_set: &mut s.case_set,
                default_case: &mut s.default_case,
                value_ty: s.value.ty.to_constant().unwrap(),
            };
            collect_switch_cases(&mut s.body, &mut data, diagnostics);
        }
        StmtKind::Case {
            value,
            label,
            header_span,
            stmt,
        } => {
            let casted_value = value.cast(data.value_ty);

            if data.case_set.contains(&casted_value) {
                diagnostics.analysis_error(
                    header_span.clone(),
                    format!("duplicate case value {}", value),
                );
            }

            data.case_set.insert(casted_value);
            data.cases.push((label.clone(), casted_value));
            collect_switch_cases(stmt, data, diagnostics);
        }
        StmtKind::DefaultCase {
            label,
            header_span,
            stmt,
        } => {
            if data.default_case.is_some() {
                diagnostics.analysis_error(
                    header_span.clone(),
                    "multiple 'default' cases in one switch",
                );
            } else {
                *data.default_case = Some(label.clone());
            }

            collect_switch_cases(stmt, data, diagnostics);
        }
        _ => {}
    }
}
