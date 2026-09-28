use std::fmt;

use ariadne::{Color, Report, ReportKind, Source};

use crate::core::LineMap;

pub use crate::core::Span;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Primary,
    Secondary,
}

#[derive(Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub kind: LabelKind,
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

    pub fn add(&mut self, stage: Stage, span: Span, message: impl Into<String>) -> &mut Diagnostic {
        let diagnostic = Diagnostic::new(stage, span, message);
        self.vec.push(diagnostic);
        self.vec.last_mut().expect("just pushed")
    }

    pub fn lexing_error(&mut self, span: Span, message: impl ToString) -> &mut Diagnostic {
        self.add(Stage::Lex, span, message.to_string())
    }

    pub fn parsing_error(&mut self, span: Span, message: impl ToString) -> &mut Diagnostic {
        self.add(Stage::Parse, span, message.to_string())
    }

    pub fn analysis_error(&mut self, span: Span, message: impl ToString) -> &mut Diagnostic {
        self.add(Stage::Analysis, span, message.to_string())
    }
}

#[derive(Debug)]
pub struct Diagnostic {
    pub stage: Stage,
    pub message: String,
    pub labels: Vec<Label>,
}

impl Diagnostic {
    pub fn new(stage: Stage, span: Span, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
            labels: vec![Label {
                span,
                message: String::new(),
                kind: LabelKind::Primary,
            }],
        }
    }

    pub fn and_label(&mut self, span: Span, message: impl Into<String>) -> &mut Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            kind: LabelKind::Secondary,
        });
        self
    }

    pub fn span(&self) -> Span {
        self.labels[0].span.clone()
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

pub fn report_all(file_name: &str, source: &str, line_map: &LineMap, diagnostics: &[Diagnostic]) {
    let rendered = format!("{file_name} (preprocessed)");

    for d in diagnostics {
        let offset = d.span().start;

        let where_ = match line_map.locate(source, offset) {
            Some((file, line, col)) => format!("{file}:{line}:{col}"),
            None => {
                let (line, col) = line_col(source, offset);
                format!("{file_name}:{line}:{col}")
            }
        };

        let mut report = Report::build(ReportKind::Error, (&rendered, d.span()))
            .with_message(format!("{} error: {} ({where_})", d.stage, d.message));

        for label in &d.labels {
            let mut message = if label.message.is_empty() {
                d.message.clone()
            } else {
                label.message.clone()
            };

            let colour = match label.kind {
                LabelKind::Primary => {
                    if let Some((file, line, _)) = line_map.locate(source, label.span.start) {
                        message.push_str(&format!(" ({file}:{line})"));
                    }
                    Color::Red
                }
                LabelKind::Secondary => {
                    if let Some((file, line, _)) = line_map.locate(source, label.span.start) {
                        message.push_str(&format!(" ({file}:{line})"));
                    }
                    Color::Blue
                }
            };

            report = report.with_label(
                ariadne::Label::new((&rendered, label.span.clone()))
                    .with_message(message)
                    .with_color(colour),
            );
        }

        let _ = report.finish().print((&rendered, Source::from(source)));
    }
}
