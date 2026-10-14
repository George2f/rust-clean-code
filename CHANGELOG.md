# Changelog

All notable changes to this project are documented in this file.

## [0.2.0] - 2026-10-15

### Changed

- Rewritten README: Clean Code tagline, install section, a section for coding agents (JSON
  output, exit codes, autofix scope) and the rule table kept in place.

## [0.1.0] - 2026-10-15

### Added

- `clean-code-lints`: all 44 Clean Code rules from the
  [clean-code ESLint plugin](https://github.com/george2f/eslint-plugin-clean-code) ported to Rust,
  with the same Clean Code identifiers, `recommended` / `strict` / `strictest` presets and the same
  12 static / 32 heuristic split.
- `cargo-clean-code`: CLI (`cargo clean-code`) with pretty and JSON output, `--preset`,
  per-rule `--allow` / `--warn` / `--deny` overrides and `--fix` for mechanically safe fixes.
- `clean-code.toml` configuration with per-rule levels and options.
- The repository lints itself with the `strictest` preset (every rule at error) in CI, matching the
  ESLint plugin's self-linting discipline.
- Rule-by-rule Rust adaptation notes in [MAPPING.md](MAPPING.md).
