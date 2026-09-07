use std::collections::HashSet;

use crate::{
    analysis::errors::SemanticError,
    parser::{BlockItem, Program, StmtKind},
};

pub fn rename_all_gotos(program: &mut Program) {
    for func in &mut program.0 {
        if let Some(body) = &mut func.body {
            for item in body {
                match item {
                    BlockItem::Stmt(s) => {
                        s.process_inner_statements_mut(&mut (), &|stmt, _| match &mut stmt.kind {
                            StmtKind::Goto(i) => {
                                *i = i.with_suffix(format!(".{}", func.name.0));
                            }
                            StmtKind::Label(i, _) => {
                                *i = i.with_suffix(format!(".{}", func.name.0));
                            }
                            _ => (),
                        });
                    }
                    _ => (),
                }
            }
        }
    }
}

pub fn check_if_all_gotos_point_somewhere_valid(
    program: &Program,
    errors: &mut Vec<SemanticError>,
) {
    for func in &program.0 {
        if let Some(body) = &func.body {
            let mut labels = HashSet::new();

            for item in body {
                match item {
                    BlockItem::Stmt(s) => {
                        s.process_inner_statements(
                            &mut (&mut labels, &mut *errors),
                            &|stmt, (labels, errors)| match &stmt.kind {
                                StmtKind::Label(i, _) => {
                                    if labels.contains(i) {
                                        errors.push(SemanticError::DuplicateLabel {
                                            span: stmt.span.clone(),
                                            name: i.1.clone(),
                                        });
                                    } else {
                                        labels.insert(i.clone());
                                    }
                                }
                                _ => (),
                            },
                        );
                    }
                    _ => (),
                }
            }

            for item in body {
                match item {
                    BlockItem::Stmt(s) => {
                        s.process_inner_statements(errors, &|stmt, errors| match &stmt.kind {
                            StmtKind::Goto(i) => {
                                if !labels.contains(i) {
                                    errors.push(SemanticError::UndeclaredGotoTarget {
                                        span: stmt.span.clone(),
                                        name: i.1.clone(),
                                    });
                                }
                            }
                            _ => (),
                        });
                    }
                    _ => (),
                }
            }
        }
    }
}
