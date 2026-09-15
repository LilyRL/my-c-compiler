use crate::{
    diagnostics::Diagnostics,
    parser::{
        BlockItem, Constant, Declaration, ExprKind, Expression, Program, Statement, StmtKind,
    },
};

pub fn check_for_nested_functions(program: &Program, diagnostics: &mut Diagnostics) {
    fn check(item: &Declaration, diagnostics: &mut Diagnostics) {
        match item {
            Declaration::Func(func) => {
                if func.body.is_some() {
                    diagnostics.analysis_error(
                        func.span.clone(),
                        format!(
                            "function '{}' cannot be declared inside another function",
                            func.name.1
                        ),
                    );
                }
            }
            _ => (),
        }
    }

    for func in program.functions() {
        if let Some(body) = &func.body {
            for item in body {
                match &item {
                    BlockItem::Decl(d) => {
                        check(d, diagnostics);
                    }
                    BlockItem::Stmt(stmt) => {
                        stmt.process_inner_declarations(diagnostics, &check);
                    }
                }
            }
        }
    }
}

pub fn add_return_zero(program: &mut Program) {
    for function in program.functions_mut() {
        if let Some(block) = &mut function.body {
            if block.last().is_some_and(|item| {
                let BlockItem::Stmt(s) = item else {
                    return false;
                };

                matches!(s.kind, StmtKind::Return(_))
            }) {
                continue;
            }

            block.push(BlockItem::Stmt(Statement::new(
                StmtKind::Return(Expression::new(ExprKind::Constant(Constant::Int(0)), 0..0)),
                0..0,
            )));
        }
    }
}
