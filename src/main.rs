use std::{
    fs,
    path::PathBuf,
    process::{Command, ExitCode, Stdio},
};

use clap::Parser;

use diagnostics::{Diagnostic, report_all};
use lexer::lex;
use parser::parse;

use crate::analysis::validate_program;

mod analysis;
mod codegen;
mod diagnostics;
mod ir;
mod lexer;
mod parser;

#[derive(Parser, Debug)]
struct Args {
    /// Path to C file to be compiled
    input_path: PathBuf,

    /// Path to output executable
    #[arg(short = 'o', long = "output")]
    output_path: Option<PathBuf>,

    /// Stop after lexing
    #[arg(long, default_value_t = false)]
    lex: bool,

    /// Stop after parsing
    #[arg(long, default_value_t = false)]
    parse: bool,

    /// Stop after semantic analysis
    #[arg(long, default_value_t = false)]
    validate: bool,

    /// Stop after creating IR and print it out
    #[arg(long, default_value_t = false)]
    tacky: bool,

    /// Stop after codegen without emitting assembly
    #[arg(long, default_value_t = false)]
    codegen: bool,

    /// Emit assembly file, but do not assemble or link
    #[arg(short = 'S', default_value_t = false)]
    dont_assemble: bool,

    /// Keep intermediate .s files
    #[arg(long, default_value_t = false)]
    keep_intermediates: bool,

    #[arg(short = 'c', default_value_t = false)]
    generate_object: bool,
}

#[derive(Debug)]
struct Paths {
    input: PathBuf,
    assembly: PathBuf,
    output: PathBuf,
    ir: PathBuf,
    parsed_ast: PathBuf,
    tokens: PathBuf,
}

impl Paths {
    fn new(args: &Args) -> Self {
        let input = args.input_path.clone();
        let assembly = input.with_extension("s");
        let ir = input.with_extension("ir");
        let parsed_ast = input.with_extension("ast");
        let tokens = input.with_extension("tokens");

        let output = if let Some(path) = args.output_path.clone() {
            path
        } else {
            if args.generate_object {
                input.with_extension("o")
            } else {
                input.with_extension("")
            }
        };

        Self {
            input,
            assembly,
            output,
            ir,
            parsed_ast,
            tokens,
        }
    }
}

fn compile_pipeline(
    source: &str,
    args: &Args,
    paths: &Paths,
) -> Result<Option<String>, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    if args.lex {
        println!("{:#?}", tokens);
        return Ok(None);
    }

    if args.keep_intermediates {
        let _ = fs::write(&paths.tokens, format!("{:#?}", tokens));
    }

    let mut program = parse(source.to_string(), tokens)?;
    if args.parse {
        println!("{:#?}", program);
        return Ok(None);
    }

    {
        let mut diagnostics = diagnostics::Diagnostics::new();
        validate_program(&mut program, &mut diagnostics);

        if args.keep_intermediates {
            let _ = fs::write(&paths.parsed_ast, format!("{:#?}", program));
        }

        if !diagnostics.is_empty() {
            return Err(diagnostics.vec);
        }
    }

    if args.validate {
        return Ok(None);
    }

    let tacky_program = program.lower();
    if args.tacky {
        println!("{tacky_program}");
        return Ok(None);
    }

    if args.keep_intermediates {
        let _ = fs::write(&paths.ir, format!("{tacky_program}"));
    }

    let mut asm_program = tacky_program.lower();
    codegen::transform(&mut asm_program);

    if args.codegen {
        println!("{:#?}", asm_program);

        return Ok(None);
    }

    Ok(Some(asm_program.format()))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let paths = Paths::new(&args);
    let file_name = paths.input.to_string_lossy();

    let preproc = Command::new("gcc")
        .args(["-E", "-P"])
        .arg(&paths.input)
        .stdout(Stdio::piped())
        .output()?;

    if !preproc.status.success() {
        return Err(format!(
            "Preprocessing step (gcc -E) failed:\n{}",
            String::from_utf8_lossy(&preproc.stderr)
        )
        .into());
    }

    let preprocessed_source = String::from_utf8(preproc.stdout)?;

    let asm_output = match compile_pipeline(&preprocessed_source, &args, &paths) {
        Err(diagnostics) => {
            report_all(&file_name, &preprocessed_source, &diagnostics);
            return Err("compilation failed".into());
        }
        Ok(None) => return Ok(()),
        Ok(Some(asm_output)) => asm_output,
    };

    fs::write(&paths.assembly, asm_output)?;

    if args.dont_assemble {
        return Ok(());
    }

    let mut assemble_cmd = Command::new("gcc");
    assemble_cmd.arg(&paths.assembly);

    if args.generate_object {
        assemble_cmd.arg("-c");
    }

    assemble_cmd.args(["-o"]).arg(&paths.output);

    let assemble_status = assemble_cmd.status()?;

    if !args.keep_intermediates {
        let _ = fs::remove_file(&paths.assembly);
    }

    if !assemble_status.success() {
        return Err("Assembly/linking step (gcc) failed".into());
    }

    Ok(())
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
