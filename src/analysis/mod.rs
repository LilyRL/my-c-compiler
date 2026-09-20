use crate::{
    analysis::{
        functions::{add_return_zero, check_for_nested_functions},
        goto::{check_if_all_gotos_point_somewhere_valid, rename_all_gotos},
        typechecking::check_all_types,
    },
    diagnostics::Diagnostics,
    parser::{BlockItem, ExprKind, Expression, Program, VariableDeclaration},
};

use declarations::resolve_all_identifiers;
use loops::label_all_loops;
use switch::collect_all_switch_cases;

pub use declarations::get_identifiers;
pub use typechecking::{IdentifierAttributes, InitialValue, StaticInit, Symbol, Type, get_symbols};

mod declarations;
mod functions;
mod goto;
mod loops;
mod switch;
mod typechecking;

pub fn validate_program(program: &mut Program, diagnostics: &mut Diagnostics) {
    add_return_zero(program);
    replace_compound_assign_with_assign_binary(program);
    resolve_all_identifiers(program, diagnostics);
    label_all_loops(program, diagnostics);
    rename_all_gotos(program);
    check_if_all_gotos_point_somewhere_valid(program, diagnostics);
    check_all_types(program, diagnostics);
    collect_all_switch_cases(program, diagnostics);
    check_for_nested_functions(program, diagnostics);
}

fn replace_compound_assign_with_assign_binary(program: &mut Program) {
    fn f(expr: &mut Expression, _: &mut ()) {
        match expr.kind {
            ExprKind::CompoundAssign {
                operator,
                ref lhs,
                ref rhs,
            } => {
                *expr = Expression {
                    ty: Type::Int,
                    span: expr.span.clone(),
                    kind: ExprKind::Assignment(
                        lhs.clone(),
                        Box::new(Expression {
                            ty: Type::Int,
                            kind: ExprKind::Binary {
                                operator: operator.compound_assign().unwrap(),
                                lhs: lhs.clone(),
                                rhs: rhs.clone(),
                            },
                            span: expr.span.clone(),
                        }),
                    ),
                };
            }
            _ => (),
        }
    }

    for function in program.functions_mut() {
        if let Some(body) = &mut function.body {
            for block_item in body.iter_mut() {
                match block_item {
                    BlockItem::Stmt(s) => s.process_inner_expressions_mut(&mut (), &f),
                    BlockItem::Decl(d) => match d {
                        crate::parser::Declaration::Var(VariableDeclaration {
                            init: Some(init),
                            ..
                        }) => {
                            init.process_inner_expressions_mut(&mut (), &f);
                        }
                        _ => (),
                    },
                }
            }
        }
    }
}
