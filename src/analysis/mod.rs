use crate::{
    analysis::{
        functions::{add_return_zero, check_for_nested_functions},
        goto::{check_if_all_gotos_point_somewhere_valid, rename_all_gotos},
        typechecking::check_all_types,
    },
    parser::Program,
};

use declarations::resolve_all_identifiers;
use loops::label_all_loops;
use switch::collect_all_switch_cases;

pub use declarations::get_identifiers;

use errors::SemanticError;

mod declarations;
mod errors;
mod functions;
mod goto;
mod loops;
mod switch;
mod typechecking;

pub fn validate_program(program: &mut Program) -> Vec<SemanticError> {
    add_return_zero(program);

    let mut errors = Vec::new();
    resolve_all_identifiers(program, &mut errors);
    label_all_loops(program, &mut errors);
    rename_all_gotos(program);
    check_if_all_gotos_point_somewhere_valid(program, &mut errors);
    collect_all_switch_cases(program, &mut errors);
    check_all_types(program, &mut errors);
    check_for_nested_functions(program, &mut errors);

    errors
}
