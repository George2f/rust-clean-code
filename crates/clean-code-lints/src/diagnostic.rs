use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warn => "warn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub edits: Vec<TextEdit>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub help: Option<String>,
    pub file: PathBuf,
    pub start: Position,
    pub end: Position,
    pub fix: Option<Fix>,
}

impl Diagnostic {
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}
