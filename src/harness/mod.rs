use std::{
    fs,
    process::{Command, Stdio},
};

use args::{Args, Paths};
use clap::Parser;

use crate::{
    analysis::validate_program,
    asm,
    core::LineMap,
    diagnostics::{Diagnostic, Diagnostics, report_all},
    lexer::lex,
    syntax::parse,
    target::handle_target_os_arguement,
};

mod args;

type Error = Box<dyn std::error::Error>;
type Res<T> = Result<T, Error>;

pub fn run() -> Res<()> {
    let args = Args::parse();
    handle_target_os_arguement(&args.target_os);

    let paths = Paths::new(&args);
    let file_name = paths.input.to_string_lossy();
    let preprocessed_source = preprocess_source(&paths)?;
    let (line_map, display) = LineMap::new(&preprocessed_source);

    let asm_output = match compile_pipeline(&display, &args, &paths) {
        Err(diagnostics) => {
            report_all(&file_name, &display, &line_map, &diagnostics);
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
        let _ = fs::write(
            &paths.tokens,
            tokens
                .iter()
                .map(|t| format!("{} ", t.token))
                .collect::<String>(),
        );
    }

    let mut program = parse(source.to_string(), tokens)?;
    if args.parse {
        println!("{:#?}", program);
        return Ok(None);
    }

    let mut diagnostics = Diagnostics::new();
    let Some(mut symbols) = validate_program(&mut program, &mut diagnostics) else {
        return Err(diagnostics.vec);
    };

    if args.keep_intermediates {
        let _ = fs::write(&paths.parsed_ast, format!("{:#?}", program));
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics.vec);
    }

    if args.validate {
        return Ok(None);
    }

    let tacky_program = program.lower(&mut symbols);
    if args.tacky {
        println!("{tacky_program}");
        return Ok(None);
    }

    if args.keep_intermediates {
        let _ = fs::write(&paths.ir, format!("{tacky_program}"));
        let _ = fs::write(&paths.ir_extra, format!("{:#?}", tacky_program));
    }

    let mut asm_program = tacky_program.lower(&symbols);
    asm::transform(&mut asm_program, &symbols);

    if args.codegen {
        println!("{:#?}", asm_program);

        return Ok(None);
    }

    Ok(Some(asm_program.format(&symbols)))
}

fn preprocess_source(paths: &Paths) -> Res<String> {
    let preproc = Command::new("gcc")
        .args(["-E"])
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

    Ok(String::from_utf8(preproc.stdout)?)
}
