use std::process::ExitCode;

mod analysis;
mod asm;
mod core;
mod diagnostics;
mod harness;
mod ir;
mod lexer;
mod syntax;
mod target;

fn main() -> ExitCode {
    if let Err(err) = harness::run() {
        eprintln!("Error: {err}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
