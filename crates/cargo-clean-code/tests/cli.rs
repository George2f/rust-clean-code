use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clean-code-cli-{}-{name}", std::process::id()));
    std::fs::create_dir_all(dir.join("src"))
        .unwrap_or_else(|error| panic!("create fixture: {error}"));
    dir
}

fn run_cli(dir: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-clean-code"));
    command.args(extra).arg(dir.join("src"));
    command
        .output()
        .unwrap_or_else(|error| panic!("run cargo-clean-code: {error}"))
}

#[test]
fn reports_magic_number_as_json_and_exits_one() {
    let dir = fixture_dir("magic");
    std::fs::write(dir.join("src/lib.rs"), "pub fn answer() -> i32 { 42 }\n")
        .unwrap_or_else(|error| panic!("write fixture: {error}"));
    let output = run_cli(&dir, &["--format", "json"]);
    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("utf8: {error}"));
    assert_eq!(
        (
            output.status.code(),
            stdout.contains("\"rule\":\"no-magic-numbers\"")
        ),
        (Some(1), true),
        "stdout: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clean_project_exits_zero() {
    let dir = fixture_dir("clean");
    std::fs::write(
        dir.join("src/lib.rs"),
        "const LIMIT: i32 = 42;\npub fn limit() -> i32 { LIMIT }\n",
    )
    .unwrap_or_else(|error| panic!("write fixture: {error}"));
    let output = run_cli(&dir, &["--format", "json"]);
    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("utf8: {error}"));
    assert_eq!(
        (output.status.code(), stdout.trim()),
        (Some(0), "[]"),
        "stdout: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn accepts_cargo_subcommand_invocation_with_separator() {
    let dir = fixture_dir("cargo");
    std::fs::write(dir.join("src/lib.rs"), "pub fn answer() -> i32 { 42 }\n")
        .unwrap_or_else(|error| panic!("write fixture: {error}"));
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-clean-code"))
        .args(["clean-code", "--", "--format", "json"])
        .arg(dir.join("src"))
        .output()
        .unwrap_or_else(|error| panic!("run cargo-clean-code: {error}"));
    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("utf8: {error}"));
    assert_eq!(
        (output.status.code(), stdout.contains("no-magic-numbers")),
        (Some(1), true),
        "stdout: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rejects_missing_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-clean-code"))
        .arg("/nope/does-not-exist")
        .output()
        .unwrap_or_else(|error| panic!("run cargo-clean-code: {error}"));
    let stderr = String::from_utf8(output.stderr).unwrap_or_else(|error| panic!("utf8: {error}"));
    assert_eq!(
        (output.status.code(), stderr.contains("path does not exist")),
        (Some(2), true),
        "stderr: {stderr}"
    );
}
