use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
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

    /// Stop after codegen without emitting assembly
    #[arg(long, default_value_t = false)]
    codegen: bool,

    /// Emit assembly file, but do not assemble or link
    #[arg(short = 'S', default_value_t = false)]
    dont_assemble: bool,

    /// Keep intermediate .i and .s files
    #[arg(long, default_value_t = false)]
    keep_intermediates: bool,
}

#[derive(Debug)]
struct Paths {
    input: PathBuf,
    preprocessed: PathBuf,
    assembly: PathBuf,
    output: PathBuf,
}

impl Paths {
    fn new(args: &Args) -> Self {
        let input = args.input_path.clone();
        let stem = input.file_stem().expect("Input file has no valid filename");

        let preprocessed = input.with_extension("i");
        let assembly = input.with_extension("s");

        let output = args
            .output_path
            .clone()
            .unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new("")).join(stem));

        Self {
            input,
            preprocessed,
            assembly,
            output,
        }
    }
}

struct TempFileGuard {
    path: PathBuf,
    keep: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf, keep: bool) -> Self {
        Self { path, keep }
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.keep && self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn compile_pipeline(source: &str, args: &Args) -> Result<Option<String>, String> {
    let tokens = lex(source).ok_or("Lexing failed")?;
    dbg!(&tokens);
    if args.lex {
        return Ok(None);
    }

    let program = parse(source.to_string(), tokens).ok_or("Parsing failed")?;
    dbg!(&program);
    if args.parse {
        return Ok(None);
    }

    let asm_program = program.lower();
    dbg!(&asm_program);

    Ok(Some(asm_program.format()))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let paths = Paths::new(&args);
    dbg!(&paths);

    let preproc_status = Command::new("gcc")
        .args([
            "-E",
            "-P",
            paths.input.to_str().ok_or("Invalid input path string")?,
            "-o",
            paths
                .preprocessed
                .to_str()
                .ok_or("Invalid preprocessed path string")?,
        ])
        .status()?;

    if !preproc_status.success() {
        return Err("Preprocessing step (gcc -E) failed".into());
    }

    let _i_guard = TempFileGuard::new(paths.preprocessed.clone(), args.keep_intermediates);

    let preprocessed_source = fs::read_to_string(&paths.preprocessed)?;

    if args.codegen {
        return Ok(());
    }

    let asm_output = match compile_pipeline(&preprocessed_source, &args)? {
        Some(asm) => asm,
        None => return Ok(()),
    };

    fs::write(&paths.assembly, asm_output)?;

    let _s_guard = TempFileGuard::new(paths.assembly.clone(), args.keep_intermediates);

    if args.dont_assemble {
        return Ok(());
    }

    let assemble_status = Command::new("gcc")
        .args([
            paths
                .assembly
                .to_str()
                .ok_or("Invalid assembly path string")?,
            "-o",
            paths.output.to_str().ok_or("Invalid output path string")?,
        ])
        .status()?;

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
