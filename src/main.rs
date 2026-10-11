#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::process::ExitCode;

mod analysis;
mod asm;
mod core;
mod diagnostics;
mod harness;
mod tacky;
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
