use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::diagnostic::{Diagnostic, Severity};
use crate::project::{FileEntry, ParseError, ProjectIndex};
use crate::rule::{Finding, Level, Rule, RuleContext};
use crate::rules;
use crate::source::SourceFile;

pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub files_checked: usize,
    pub parse_errors: Vec<ParseError>,
}

impl Report {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| !diagnostic.is_error())
            .count()
    }

    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty() && self.parse_errors.is_empty()
    }
}

pub fn analyze_paths(paths: &[PathBuf], config: &Config) -> Report {
    let (sources, read_errors) = read_sources(paths);
    let mut report = analyze_sources(&sources, config);
    report.parse_errors.extend(read_errors);
    report.parse_errors = sorted_parse_errors(std::mem::take(&mut report.parse_errors));
    report
}

pub fn analyze_sources(sources: &[(PathBuf, String)], config: &Config) -> Report {
    let (entries, parse_errors) = parse_entries(sources);
    let files_checked = entries.len();
    let project = ProjectIndex::build(entries);
    let diagnostics = sorted_diagnostics(run_rules(&project, config));
    let parse_errors = sorted_parse_errors(parse_errors);
    Report {
        diagnostics,
        files_checked,
        parse_errors,
    }
}

fn read_sources(paths: &[PathBuf]) -> (Vec<(PathBuf, String)>, Vec<ParseError>) {
    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for file in collect_rust_files(paths) {
        match read_source(file) {
            Ok(source) => sources.push(source),
            Err(error) => errors.push(error),
        }
    }
    (sources, errors)
}

fn read_source(file: PathBuf) -> Result<(PathBuf, String), ParseError> {
    let text = std::fs::read_to_string(&file).map_err(|error| ParseError {
        file: file.clone(),
        message: error.to_string(),
    })?;
    Ok((file, text))
}

fn run_rules(project: &ProjectIndex, config: &Config) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for rule in rules::all() {
        let rule = rule.as_ref();
        diagnostics.extend(rule_diagnostics(rule, project, config));
    }
    diagnostics
}

fn rule_diagnostics(rule: &dyn Rule, project: &ProjectIndex, config: &Config) -> Vec<Diagnostic> {
    let meta = rule.meta();
    let level = config.effective_level(meta);
    if level == Level::Allow {
        return Vec::new();
    }
    let severity = severity_for(level);
    let options = config.rule_options(meta.name);
    let mut diagnostics = Vec::new();
    for entry in &project.entries {
        let ctx = RuleContext {
            file: &entry.source,
            ast: &entry.ast,
            options: &options,
            project,
        };
        for finding in rule.check(&ctx) {
            diagnostics.push(to_diagnostic(meta.name, severity, finding));
        }
    }
    for finding in rule.check_project(project, &options) {
        diagnostics.push(to_diagnostic(meta.name, severity, finding));
    }
    diagnostics
}

fn sorted_diagnostics(mut items: Vec<Diagnostic>) -> Vec<Diagnostic> {
    items.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.start.line.cmp(&right.start.line))
            .then(left.start.column.cmp(&right.start.column))
            .then(left.rule.cmp(right.rule))
    });
    items
}

fn sorted_parse_errors(mut errors: Vec<ParseError>) -> Vec<ParseError> {
    errors.sort_by(|left, right| left.file.cmp(&right.file));
    errors
}

pub fn collect_rust_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        files.extend(collect(path));
    }
    files.sort();
    files.dedup();
    files
}

fn collect(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        let is_rust = path.extension().is_some_and(|extension| extension == "rs");
        return if is_rust {
            vec![path.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    if !path.is_dir() || is_ignored_dir(path) {
        return Vec::new();
    }
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .flat_map(|entry| collect(&entry.path()))
                .collect()
        })
        .unwrap_or_default()
}

fn is_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().and_then(|name| name.to_str());
    matches!(name, Some("target" | ".git"))
}

pub(crate) fn parse_entries(sources: &[(PathBuf, String)]) -> (Vec<FileEntry>, Vec<ParseError>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for (path, text) in sources {
        match parse_entry(path, text) {
            Ok(entry) => entries.push(entry),
            Err(error) => errors.push(error),
        }
    }
    (entries, errors)
}

fn parse_entry(path: &Path, text: &str) -> Result<FileEntry, ParseError> {
    let ast = syn::parse_file(text).map_err(|error| ParseError {
        file: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let source = SourceFile::new(path.to_path_buf(), text.to_string());
    Ok(FileEntry {
        path: path.to_path_buf(),
        source,
        ast,
    })
}

fn severity_for(level: Level) -> Severity {
    match level {
        Level::Error => Severity::Error,
        Level::Warn | Level::Allow => Severity::Warn,
    }
}

fn to_diagnostic(rule: &'static str, severity: Severity, finding: Finding) -> Diagnostic {
    Diagnostic {
        rule,
        severity,
        message: finding.message,
        help: finding.help,
        file: finding.file,
        start: finding.span.start,
        end: finding.span.end,
        fix: finding.fix,
    }
}
