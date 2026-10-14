use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use clean_code_lints::{analyze_paths, Config, Diagnostic, Level, Preset, Report, TextEdit};

#[derive(Parser)]
#[command(
    name = "cargo-clean-code",
    bin_name = "cargo clean-code",
    version,
    about = "Clean Code lints for Rust, derived from the clean-code ESLint plugin"
)]
struct Cli {
    /// Files or directories to check (default: current directory)
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
    /// Rule preset to apply: recommended, strict, or strictest
    #[arg(long, value_name = "PRESET")]
    preset: Option<String>,
    /// Output format: pretty or json
    #[arg(long, value_name = "FORMAT", default_value = "pretty")]
    format: String,
    /// Apply the fixes that are mechanically safe
    #[arg(long)]
    fix: bool,
    /// Turn a rule off (repeatable)
    #[arg(long, value_name = "RULE")]
    allow: Vec<String>,
    /// Set a rule to warn (repeatable)
    #[arg(long, value_name = "RULE")]
    warn: Vec<String>,
    /// Set a rule to error (repeatable)
    #[arg(long, value_name = "RULE")]
    deny: Vec<String>,
    /// Path to the configuration file (default: ./clean-code.toml)
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = strip_cargo_args(std::env::args_os().collect());
    let cli = Cli::parse_from(args);
    match run(&cli) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn strip_cargo_args(args: Vec<OsString>) -> Vec<OsString> {
    strip_leading(strip_leading(args, "clean-code"), "--")
}

fn strip_leading(mut args: Vec<OsString>, value: &str) -> Vec<OsString> {
    let is_present = args.get(1).is_some_and(|argument| argument == value);
    if is_present {
        args.remove(1);
    }
    args
}

fn run(cli: &Cli) -> Result<ExitCode, String> {
    let format = cli.format.as_str();
    validate_format(format)?;
    let config = build_config(cli)?;
    let paths = collect_paths(&cli.paths)?;
    if cli.fix {
        return fix_analysis(&paths, &config, format);
    }
    let report = analyze_paths(&paths, &config);
    print_report(&report, format);
    Ok(exit_code(&report))
}

fn validate_format(format: &str) -> Result<(), String> {
    let known = matches!(format, "pretty" | "json");
    if known {
        return Ok(());
    }
    Err(format!(
        "unknown format `{format}`; expected pretty or json"
    ))
}

fn build_config(cli: &Cli) -> Result<Config, String> {
    let config = load_config(cli)?;
    let config = apply_preset(config, &cli.preset)?;
    let config = with_override(config, &cli.allow, Level::Allow)?;
    let config = with_override(config, &cli.warn, Level::Warn)?;
    let config = with_override(config, &cli.deny, Level::Error)?;
    Ok(config)
}

fn load_config(cli: &Cli) -> Result<Config, String> {
    let explicit = &cli.config;
    if let Some(path) = explicit {
        return Config::from_file(path);
    }
    let default = Path::new("clean-code.toml");
    if default.is_file() {
        return Config::from_file(default);
    }
    Ok(Config::default())
}

fn apply_preset(config: Config, preset: &Option<String>) -> Result<Config, String> {
    let Some(preset) = preset else {
        return Ok(config);
    };
    let parsed = Preset::parse(preset).ok_or_else(|| {
        format!("unknown preset `{preset}`; expected recommended, strict, or strictest")
    })?;
    let mut config = config;
    config.set_preset(parsed);
    Ok(config)
}

fn with_override(config: Config, rules: &[String], level: Level) -> Result<Config, String> {
    let mut config = config;
    for rule in rules {
        let known = clean_code_lints::rules::by_name(rule);
        if known.is_none() {
            return Err(format!("unknown rule `{rule}`"));
        }
        config.set_level(rule, level);
    }
    Ok(config)
}

fn collect_paths(configured: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let paths = default_paths(configured);
    for path in &paths {
        if !path.exists() {
            return Err(format!("path does not exist: {}", path.display()));
        }
    }
    Ok(paths)
}

fn default_paths(configured: &[PathBuf]) -> Vec<PathBuf> {
    if configured.is_empty() {
        return vec![PathBuf::from(".")];
    }
    configured.to_vec()
}

fn fix_analysis(paths: &[PathBuf], config: &Config, format: &str) -> Result<ExitCode, String> {
    let report = analyze_paths(paths, config);
    let (fixes, fixed) = collect_fixes(&report);
    apply_fixes(&fixes)?;
    report_fixed(fixed, format);
    let remaining = analyze_paths(paths, config);
    print_report(&remaining, format);
    Ok(exit_code(&remaining))
}

fn collect_fixes(report: &Report) -> (BTreeMap<PathBuf, Vec<TextEdit>>, usize) {
    let mut fixes: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();
    let mut fixed = 0usize;
    for diagnostic in &report.diagnostics {
        let Some(fix) = &diagnostic.fix else {
            continue;
        };
        fixed += 1;
        let edits = fixes.entry(diagnostic.file.clone()).or_default();
        edits.extend(fix.edits.iter().cloned());
    }
    (fixes, fixed)
}

fn apply_fixes(fixes: &BTreeMap<PathBuf, Vec<TextEdit>>) -> Result<(), String> {
    for (file, edits) in fixes {
        apply_file_fixes(file, edits)?;
    }
    Ok(())
}

fn apply_file_fixes(file: &Path, edits: &[TextEdit]) -> Result<(), String> {
    let mut text =
        std::fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
    let applied = select_non_overlapping(edits);
    for edit in applied.into_iter().rev() {
        let replacement = edit.replacement.as_str();
        let in_bounds = edit.end <= text.len()
            && text.is_char_boundary(edit.start)
            && text.is_char_boundary(edit.end);
        if in_bounds {
            text.replace_range(edit.start..edit.end, replacement);
        }
    }
    std::fs::write(file, &text).map_err(|error| format!("{}: {error}", file.display()))
}

fn select_non_overlapping(edits: &[TextEdit]) -> Vec<TextEdit> {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| edit.start);
    let mut applied = Vec::new();
    let mut last_end = 0usize;
    for edit in sorted {
        if edit.start >= last_end {
            last_end = edit.end;
            applied.push(edit.clone());
        }
    }
    applied
}

fn report_fixed(fixed: usize, format: &str) {
    if fixed == 0 {
        return;
    }
    let message = format!("fixed {fixed} problem(s)");
    if format == "json" {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}

fn print_report(report: &Report, format: &str) {
    match format {
        "json" => println!("{}", to_json(report)),
        _ => print_pretty(report),
    }
}

fn print_pretty(report: &Report) {
    for error in &report.parse_errors {
        eprintln!(
            "{}: error: could not parse: {}",
            error.file.display(),
            error.message
        );
    }
    for diagnostic in &report.diagnostics {
        print_diagnostic(diagnostic);
    }
    print_summary(report);
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    let location = format!(
        "{}:{}:{}",
        diagnostic.file.display(),
        diagnostic.start.line,
        diagnostic.start.column + 1
    );
    println!(
        "{location}: {}: {} [{}]",
        diagnostic.severity.as_str(),
        diagnostic.message,
        diagnostic.rule
    );
    if let Some(help) = &diagnostic.help {
        println!("  help: {help}");
    }
}

fn print_summary(report: &Report) {
    let no_diagnostics = report.diagnostics.is_empty();
    let no_parse_errors = report.parse_errors.is_empty();
    if no_diagnostics && no_parse_errors {
        println!("clean: no problems found");
        return;
    }
    let errors = report.error_count();
    let warnings = report.warning_count();
    println!(
        "{} problem(s): {errors} error(s), {warnings} warning(s)",
        report.diagnostics.len()
    );
}

fn to_json(report: &Report) -> String {
    let items: Vec<String> = report.diagnostics.iter().map(diagnostic_json).collect();
    format!("[{}]", items.join(","))
}

fn diagnostic_json(diagnostic: &Diagnostic) -> String {
    let file = file_json(&diagnostic.file);
    let help = option_json(diagnostic.help.as_deref());
    let start = diagnostic.start;
    let end = diagnostic.end;
    let start_column = start.column + 1;
    let end_column = end.column + 1;
    format!(
        "{{\"file\":{file},\"rule\":{},\"severity\":{},\"message\":{},\"help\":{help},\"line\":{},\"column\":{start_column},\"endLine\":{},\"endColumn\":{end_column},\"fixable\":{}}}",
        json_string(diagnostic.rule),
        json_string(diagnostic.severity.as_str()),
        json_string(&diagnostic.message),
        start.line,
        end.line,
        diagnostic.fix.is_some()
    )
}

fn file_json(path: &Path) -> String {
    let text = path.display().to_string();
    json_string(&text)
}

fn option_json(help: Option<&str>) -> String {
    match help {
        Some(help) => json_string(help),
        None => "null".to_string(),
    }
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", escaped_body(value))
}

fn escaped_body(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        escaped.push_str(&escaped_char(character));
    }
    escaped
}

fn escaped_char(character: char) -> String {
    match character {
        '"' => "\\\"".to_string(),
        '\\' => "\\\\".to_string(),
        control if control.is_control() => format!("\\u{:04x}", character as u32),
        other => other.to_string(),
    }
}

fn exit_code(report: &Report) -> ExitCode {
    let errors = report.error_count();
    let parse_failed = !report.parse_errors.is_empty();
    if errors > 0 || parse_failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
