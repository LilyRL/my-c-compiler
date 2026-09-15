use crate::{
    analysis::{
        functions::{add_return_zero, check_for_nested_functions},
        goto::{check_if_all_gotos_point_somewhere_valid, rename_all_gotos},
        typechecking::check_all_types,
    },
    diagnostics::Diagnostics,
    parser::Program,
};

use declarations::resolve_all_identifiers;
use loops::label_all_loops;
use switch::collect_all_switch_cases;

pub use declarations::get_identifiers;
pub use typechecking::{IdentifierAttributes, InitialValue, Type, get_symbols};

mod declarations;
mod functions;
mod goto;
mod loops;
mod switch;
mod typechecking;

pub fn validate_program(program: &mut Program, diagnostics: &mut Diagnostics) {
    add_return_zero(program);

    resolve_all_identifiers(program, diagnostics);
    label_all_loops(program, diagnostics);
    rename_all_gotos(program);
    check_if_all_gotos_point_somewhere_valid(program, diagnostics);
    collect_all_switch_cases(program, diagnostics);
    check_all_types(program, diagnostics);
    check_for_nested_functions(program, diagnostics);
}
