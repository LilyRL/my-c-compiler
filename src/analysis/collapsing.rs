use crate::syntax::{BlockItem, ExprKind, Expression, Program};

pub fn collapse_constant_expressions(program: &mut Program) {
    for function in program.functions_mut() {
        if let Some(body) = &mut function.body {
            for item in body {
                if let BlockItem::Stmt(s) = item {
                    s.process_inner_expressions_mut(&mut (), &|e, _| collapse_if_constant(e));
                }
            }
        }
    }
}

fn collapse_if_constant(exp: &mut Expression) {
    if let Ok(new_exp) = exp.eval() {
        *exp = Expression {
            kind: ExprKind::Constant(new_exp),
            ty: new_exp.ty(),
            span: exp.span.clone(),
        }
    }
}
