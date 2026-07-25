use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, exit},
};

use clap::Parser;
use lexer::lex;
use parser::parse;

use a_lexer as lexer;
mod a_lexer;

use b_parser as parser;
mod b_parser;

use c_codegen as codegen;
mod c_codegen;

#[derive(Parser, Debug)]
struct Args {
    /// Path to C file to be compiled
    input_path: String,

    /// Path to output executable to
    #[arg(short = 'o', long)]
    output_path: Option<String>,

    /// Only run the lexer, stop before parsing
    #[arg(long, default_value_t = false)]
    lex: bool,

    /// Only run the lexer & parser, stop before codegen
    #[arg(long, default_value_t = false)]
    parse: bool,

    /// Only run the lexer, parser & codegen, stop before code emission
    #[arg(long, default_value_t = false)]
    codegen: bool,

    /// Emit assembly file, but dont assemble or link it
    #[arg(short = 'S', default_value_t = false)]
    dont_assemble: bool,

    /// Whether to keep intermediate .i & .s files.
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
        let input = Path::new(&args.input_path);
        let filename = input
            .file_prefix()
            .expect("Input file has no filename")
            .to_str()
            .expect("Input filename is invalid")
            .to_string();
        let input_dir = input
            .parent()
            .expect("Input file path has no parent directory");

        let mut preprocessed = input_dir.to_path_buf();
        preprocessed.push(&format!("{filename}.i"));

        let mut assembly = input_dir.to_path_buf();
        assembly.push(&format!("{filename}.s"));

        let output = if let Some(path) = &args.output_path {
            Path::new(path).to_path_buf()
        } else {
            let mut output = input_dir.to_path_buf();
            output.push(&filename);
            output
        };

        Self {
            input: input.to_path_buf(),
            preprocessed,
            assembly,
            output,
        }
    }
}

fn compile(source: &str, _args: &Args) -> Option<String> {
    let tokens = lex(source)?;
    dbg!(&tokens);
    let program = parse(source.to_string(), tokens);

    if program.is_none() {
        exit(1);
    }

    dbg!(&program);

    let asm_program = program.unwrap().lower();
    dbg!(&asm_program);

    return Some(asm_program.format());
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let paths = Paths::new(&args);
    dbg!(&paths);

    Command::new("gcc")
        .args([
            "-E",
            "-P",
            paths.input.to_str().unwrap(),
            "-o",
            paths.preprocessed.to_str().unwrap(),
        ])
        .output()?;

    let Ok(preprocessed_source) = fs::read_to_string(&paths.preprocessed) else {
        println!(
            "Failed to read preprocessed source file: {}",
            paths.preprocessed.display()
        );
        exit(1);
    };
    fs::write(
        &paths.assembly,
        compile(&preprocessed_source, &args).expect("Compilation failed"),
    )?;

    if !args.keep_intermediates {
        fs::remove_file(paths.preprocessed)?;
    }

    if args.dont_assemble {
        return Ok(());
    }

    Command::new("gcc")
        .args([
            paths.assembly.to_str().unwrap(),
            "-o",
            paths.output.to_str().unwrap(),
        ])
        .output()?;

    if !args.keep_intermediates {
        fs::remove_file(paths.assembly)?;
    }

    Ok(())
}
