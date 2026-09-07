use crate::{
    analysis::errors::SemanticError,
    parser::{
        BlockItem, Constant, Declaration, ExprKind, Expression, Program, Statement, StmtKind,
    },
};

pub fn check_for_nested_functions(program: &Program, errors: &mut Vec<SemanticError>) {
    fn check(item: &Declaration, errors: &mut Vec<SemanticError>) {
        match item {
            Declaration::Func(func) => {
                if func.body.is_some() {
                    errors.push(SemanticError::NestedFunction {
                        span: func.span.clone(),
                        name: func.name.1.clone(),
                    });
                }
            }
            _ => (),
        }
    }

    for func in &program.0 {
        if let Some(body) = &func.body {
            for item in body {
                match &item {
                    BlockItem::Decl(d) => {
                        check(d, errors);
                    }
                    BlockItem::Stmt(stmt) => {
                        stmt.process_inner_declarations(errors, &check);
                    }
                    _ => (),
                }
            }
        }
    }
}

pub fn add_return_zero(program: &mut Program) {
    for function in &mut program.0 {
        if let Some(block) = &mut function.body {
            block.push(BlockItem::Stmt(Statement::new(
                StmtKind::Return(Expression::new(ExprKind::Constant(Constant::Int(0)), 0..0)),
                0..0,
            )));
        }
    }
}
