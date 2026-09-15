use std::{fmt, ops::Range};

use ariadne::{Label, Report, ReportKind, Source};

pub type Span = Range<usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Lex,
    Parse,
    Analysis,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Lex => "lexer",
            Self::Parse => "parser",
            Self::Analysis => "semantic analyzer",
        })
    }
}

pub struct Diagnostics {
    pub vec: Vec<Diagnostic>,
}

#[allow(unused)]
impl Diagnostics {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.vec.push(diagnostic);
    }

    pub fn add(&mut self, stage: Stage, span: Span, message: impl ToString) {
        self.vec
            .push(Diagnostic::new(stage, span, message.to_string()));
    }

    pub fn lexing_error(&mut self, span: Span, message: impl ToString) {
        self.add(Stage::Lex, span, message);
    }

    pub fn parsing_error(&mut self, span: Span, message: impl ToString) {
        self.add(Stage::Parse, span, message);
    }

    pub fn analysis_error(&mut self, span: Span, message: impl ToString) {
        self.add(Stage::Analysis, span, message);
    }
}

#[derive(Debug)]
pub struct Diagnostic {
    pub stage: Stage,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(stage: Stage, span: Span, message: impl Into<String>) -> Self {
        Self {
            stage,
            span,
            message: message.into(),
        }
    }
}

pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

pub fn report_all(file_name: &str, source: &str, diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        let (line, col) = line_col(source, d.span.start);

        Report::build(ReportKind::Error, (file_name, d.span.clone()))
            .with_message(format!(
                "{} error: {} (line {line}, column {col})",
                d.stage, d.message
            ))
            .with_label(Label::new((file_name, d.span.clone())).with_message(&d.message))
            .finish()
            .print((file_name, Source::from(source)))
            .unwrap();
    }
}
