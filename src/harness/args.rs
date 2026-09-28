use std::{fs, path::PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    /// Path to C file to be compiled
    pub input_path: PathBuf,

    /// Path to output executable
    #[arg(short = 'o', long = "output")]
    pub output_path: Option<PathBuf>,

    /// Stop after lexing
    #[arg(long, default_value_t = false)]
    pub lex: bool,

    /// Stop after parsing
    #[arg(long, default_value_t = false)]
    pub parse: bool,

    /// Stop after semantic analysis
    #[arg(long, default_value_t = false)]
    pub validate: bool,

    /// Stop after creating IR and print it out
    #[arg(long, default_value_t = false)]
    pub tacky: bool,

    /// Stop after codegen without emitting assembly
    #[arg(long, default_value_t = false)]
    pub codegen: bool,

    /// Emit assembly file, but do not assemble or link
    #[arg(short = 'S', default_value_t = false)]
    pub dont_assemble: bool,

    /// Keep intermediate .s files
    #[arg(long, default_value_t = false)]
    pub keep_intermediates: bool,

    #[arg(short = 'c', default_value_t = false)]
    pub generate_object: bool,

    #[arg(short = 't', long = "target", default_value_t = String::from("host"))]
    pub target_os: String,
}

#[derive(Debug)]
pub struct Paths {
    pub input: PathBuf,
    pub assembly: PathBuf,
    pub output: PathBuf,
    pub ir: PathBuf,
    pub ir_extra: PathBuf,
    pub parsed_ast: PathBuf,
    pub tokens: PathBuf,
}

impl Paths {
    pub fn new(args: &Args) -> Self {
        let input = args.input_path.clone();
        let _ = fs::create_dir("output");
        let assembly = PathBuf::from("output/asm.s");
        let ir = PathBuf::from("output/tacky");
        let ir_extra = PathBuf::from("output/tacky_extra");
        let parsed_ast = PathBuf::from("output/ast");
        let tokens = PathBuf::from("output/tokens");

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
            ir_extra,
        }
    }
}
