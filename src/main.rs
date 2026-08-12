use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use clap::Parser;

use a_lexer::lex;
use b_parser::parse;

use a_lexer as lexer;
mod a_lexer;

use b_parser as parser;
mod b_parser;

use c_ir as ir;
mod c_ir;

use d_codegen as codegen;
mod d_codegen;

#[derive(Parser, Debug)]
struct Args {
    /// Path to C file to be compiled
    input_path: PathBuf,

    /// Path to output executable
    #[arg(short = 'o', long)]
    output_path: Option<PathBuf>,

    /// Stop after lexing
    #[arg(long, default_value_t = false)]
    lex: bool,

    /// Stop after parsing
    #[arg(long, default_value_t = false)]
    parse: bool,

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
}

#[derive(Debug)]
struct Paths {
    input: PathBuf,
    assembly: PathBuf,
    output: PathBuf,
}

impl Paths {
    fn new(args: &Args) -> Self {
        let input = args.input_path.clone();
        let stem = input.file_stem().expect("Input file has no valid filename");
        let assembly = input.with_extension("s");
        let output = args
            .output_path
            .clone()
            .unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new("")).join(stem));

        Self {
            input,
            assembly,
            output,
        }
    }
}

fn compile_pipeline(source: &str, args: &Args) -> Result<Option<String>, String> {
    let tokens = lex(source).ok_or("Lexing failed")?;
    if args.lex {
        println!("{:#?}", tokens);
        return Ok(None);
    }

    let program = parse(source.to_string(), tokens).ok_or("Parsing failed")?;
    if args.parse {
        println!("{:#?}", program);
        return Ok(None);
    }

    let tacky_program = program.lower();
    if args.tacky {
        println!("{tacky_program}");
        return Ok(None);
    }

    let mut asm_program = tacky_program.lower();
    codegen::transform(&mut asm_program);

    if args.codegen {
        return Ok(None);
    }

    Ok(Some(asm_program.format()))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let paths = Paths::new(&args);

    // Run preprocessor directly to stdout to avoid temporary .i files
    let preproc = Command::new("gcc")
        .args(["-E", "-P"])
        .arg(&paths.input)
        .stdout(Stdio::piped())
        .output()?;

    if !preproc.status.success() {
        return Err("Preprocessing step (gcc -E) failed".into());
    }

    let preprocessed_source = String::from_utf8(preproc.stdout)?;

    let Some(asm_output) = compile_pipeline(&preprocessed_source, &args)? else {
        return Ok(());
    };

    fs::write(&paths.assembly, asm_output)?;

    if args.dont_assemble {
        return Ok(());
    }

    let assemble_status = Command::new("gcc")
        .arg(&paths.assembly)
        .args(["-o"])
        .arg(&paths.output)
        .status()?;

    // Clean up .s intermediate unless flagged to keep
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
