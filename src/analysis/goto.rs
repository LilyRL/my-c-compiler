use std::collections::HashSet;

use crate::{
    diagnostics::Diagnostics,
    parser::{BlockItem, Program, StmtKind},
};

pub fn rename_all_gotos(program: &mut Program) {
    for func in program.functions_mut() {
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

pub fn check_if_all_gotos_point_somewhere_valid(program: &Program, diagnostics: &mut Diagnostics) {
    for func in program.functions() {
        if let Some(body) = &func.body {
            let mut labels = HashSet::new();

            for item in body {
                match item {
                    BlockItem::Stmt(s) => {
                        s.process_inner_statements(
                            &mut (&mut labels, &mut *diagnostics),
                            &|stmt, (labels, diagnostics)| match &stmt.kind {
                                StmtKind::Label(i, _) => {
                                    if labels.contains(i) {
                                        diagnostics.analysis_error(
                                            stmt.span.clone(),
                                            format!("duplicate label '{}'", i.1),
                                        );
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
                        s.process_inner_statements(diagnostics, &|stmt, diagnostics| match &stmt
                            .kind
                        {
                            StmtKind::Goto(i) => {
                                if !labels.contains(i) {
                                    diagnostics.analysis_error(
                                        stmt.span.clone(),
                                        format!("undeclared goto target '{}'", i.1),
                                    );
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
