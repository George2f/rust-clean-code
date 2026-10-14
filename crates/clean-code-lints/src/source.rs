use std::path::{Path, PathBuf};

use proc_macro2::LineColumn;
use quote::ToTokens;

use crate::diagnostic::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeSpan {
    pub start: Position,
    pub end: Position,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> SourceFile {
        let text = text.into();
        let line_starts = line_starts_of(&text);
        SourceFile {
            path: path.into(),
            text,
            line_starts,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.text
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
    }

    pub fn line_text(&self, line: usize) -> &str {
        self.lines().nth(line.saturating_sub(1)).unwrap_or("")
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn byte_offset(&self, line: usize, column: usize) -> usize {
        let index = self.line_index(line);
        let line_start = self.line_starts.get(index).copied().unwrap_or(0);
        (line_start + column).min(self.text.len())
    }

    fn line_index(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        (line - 1).min(self.line_starts.len().saturating_sub(1))
    }

    pub fn position(&self, byte: usize) -> Position {
        let byte = byte.min(self.text.len());
        let index = match self.line_starts.binary_search(&byte) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        Position {
            line: index + 1,
            column: byte - self.line_starts[index],
        }
    }

    pub fn span_at(&self, byte_start: usize, byte_end: usize) -> NodeSpan {
        NodeSpan {
            start: self.position(byte_start),
            end: self.position(byte_end),
            byte_start: byte_start.min(self.text.len()),
            byte_end: byte_end.min(self.text.len()),
        }
    }

    pub fn span_of<T: ToTokens + ?Sized>(&self, node: &T) -> NodeSpan {
        let stream = node.to_token_stream();
        let mut tokens = stream.into_iter();
        let first = tokens.next();
        let mut last = None;
        for token in tokens {
            last = Some(token.span());
        }
        match first {
            Some(first) => {
                let end = last.unwrap_or(first.span()).end();
                self.span_from_line_columns(first.span().start(), end)
            }
            None => self.span_at(0, 0),
        }
    }

    pub fn span_from_line_columns(&self, start: LineColumn, end: LineColumn) -> NodeSpan {
        let byte_start = self.byte_offset(start.line, start.column);
        let byte_end = self.byte_offset(end.line, end.column);
        self.span_at(byte_start, byte_end)
    }
}

fn line_starts_of(text: &str) -> Vec<usize> {
    let mut line_starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(index + 1);
        }
    }
    line_starts
}
