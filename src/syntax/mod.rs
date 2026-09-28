use crate::{
    diagnostics::{Diagnostic, Stage},
    lexer::SpannedToken,
};

pub use ast::*;
pub use operators::*;

mod ast;
mod display;
mod operators;
mod parser;

pub fn parse(source: String, tokens: Vec<SpannedToken>) -> Result<Program, Vec<Diagnostic>> {
    let mut parser = parser::Parser::new(source, tokens);
    let program = parser.parse();

    match program {
        Some(program) if parser.errors.is_empty() => Ok(program),
        _ => {
            if parser.errors.is_empty() {
                let span = parser.eof_span();
                parser.errors.push(Diagnostic::new(
                    Stage::Parse,
                    span,
                    "unexpected end of input",
                ));
            }
            Err(parser.errors)
        }
    }
}
