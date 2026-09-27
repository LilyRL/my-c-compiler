use std::collections::HashMap;

use crate::{
    diagnostics::{Diagnostics, Span},
    parser::{BlockItem, Identifier, Program, StmtKind},
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
            let mut labels: HashMap<Identifier, Span> = HashMap::new();

            for item in body {
                match item {
                    BlockItem::Stmt(s) => {
                        s.process_inner_statements(
                            &mut (&mut labels, &mut *diagnostics),
                            &|stmt, (labels, diagnostics)| match &stmt.kind {
                                StmtKind::Label(i, _) => {
                                    if let Some(first) = labels.get(i) {
                                        diagnostics
                                            .analysis_error(
                                                stmt.span.clone(),
                                                format!("duplicate label '{}'", i.1),
                                            )
                                            .and_label(first.clone(), "previous definition here");
                                    } else {
                                        labels.insert(i.clone(), stmt.span.clone());
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
                                if !labels.contains_key(i) {
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
