# clean-code-lints

[![CI](https://github.com/george2f/rust-clean-code/actions/workflows/ci.yml/badge.svg)](https://github.com/george2f/rust-clean-code/actions/workflows/ci.yml)
[![crates.io (clean-code-lints)](https://img.shields.io/crates/v/clean-code-lints.svg)](https://crates.io/crates/clean-code-lints)
[![crates.io (cargo-clean-code)](https://img.shields.io/crates/v/cargo-clean-code.svg)](https://crates.io/crates/cargo-clean-code)
[![docs.rs](https://img.shields.io/docsrs/clean-code-lints)](https://docs.rs/clean-code-lints)
[![license](https://img.shields.io/crates/l/clean-code-lints.svg)](LICENSE)

Clean Code lints for Rust. **Lint your code just like uncle Bob taught you.**

Every rule maps to a numbered heuristic from [Clean Code](https://www.oreilly.com/library/view/clean-code-a/9780136083238/) by Robert C. Martin, so a lint report points straight back to the book. It is the Rust sibling of the [clean-code ESLint plugin](https://github.com/George2f/eslint-plugin-clean-code) — same 44 rules, same identifiers, same presets, no JavaScript in the middle.

- 44 rules, one per book rule — 12 static, 32 heuristic
- `recommended` / `strict` / `strictest` presets, every rule tunable in `clean-code.toml`
- Stable Rust on a `syn`-based engine: no nightly, no compiler internals, no `rustc_private`
- A `cargo clean-code` CLI for CI and pre-commit, and a `clean-code-lints` library for your tests

## Why this exists

I wrote the [ESLint plugin](https://github.com/George2f/eslint-plugin-clean-code) because I wanted to know what code looks like when Clean Code is actually enforced. The answer: hard for humans, trivial for agents. This is the Rust edition — same book, same rule numbers, same presets — so the agents writing your Rust can be exactly as annoyingly consistent as the ones writing your TypeScript. Point it at `strict`, or `strictest` if you are feeling brave.

## Install

```sh
cargo install cargo-clean-code
```

The CLI is called `cargo-clean-code` so cargo can run it as a subcommand. Zero runtime dependencies beyond the AST crates — just stable Rust.

## Usage

```sh
cargo clean-code                       # recommended preset, current directory
cargo clean-code src tests --preset strict
cargo clean-code --config clean-code.toml --fix
cargo clean-code --format json          # machine-readable output
```

Exit codes: `0` clean (or warnings only), `1` errors or files that failed to parse, `2` invalid configuration or flags.

Rules can be adjusted from the command line without touching the config:

```sh
cargo clean-code --deny no-magic-numbers --allow law-of-demeter --warn no-todo-tests
```

## Configuration

Drop a `clean-code.toml` in the project root and it is picked up automatically:

```toml
preset = "strict"          # recommended (default) | strict | strictest

[rules.no-magic-numbers]
level = "error"
ignore = [-1, 0, 1, 2, 10]

[rules.law-of-demeter]
level = "warn"
maxChainLength = 4

[rules.no-todo-tests]
level = "off"
```

| Preset | Static rules | Heuristic rules |
| --- | --- | --- |
| `recommended` | error | off |
| `strict` | error | warn |
| `strictest` | error | error |

## For agents

This tool was built to be driven by coding agents as much as by people:

```sh
cargo clean-code --preset strictest --format json .   # non-interactive, deterministic
```

- **Stable names**: rule ids and Clean Code references never change between releases; use them with `--allow`, `--warn`, `--deny` or in `clean-code.toml`.
- **JSON output**: one object per finding — `file`, `rule`, `severity`, `message`, `help`, `line`, `column`, `endLine`, `endColumn`, `fixable`.
- **Exit codes**: `0` clean, `1` findings or parse failures, `2` bad configuration — safe as a CI gate.
- **Autofix**: `--fix` repairs the three mechanically safe rules (`no-negative-conditionals`, `no-dead-code`, `no-clutter`). Everything else ships a `help` string describing the intended fix.
- **Rust semantics**: read [MAPPING.md](MAPPING.md) before "fixing" a finding — it documents how each rule applies to Rust and where the approximations are.
- **Zero findings in tests**: depend on `clean-code-lints` and assert on a report (see [Library](#library)).

## Rules

The table below is the catalog; [MAPPING.md](MAPPING.md) documents the Rust semantics of every rule.

| Rule | Clean Code | Tier | Fix |
| --- | --- | --- | --- |
| `be-precise` | G26 | heuristic | |
| `command-query-separation` | Ch3 | heuristic | |
| `declaration-proximity` | G10 | heuristic | |
| `descriptive-names` | N1 | heuristic | |
| `encapsulate-boundaries` | G33 | heuristic | |
| `encapsulate-conditionals` | G28 | heuristic | |
| `function-does-one-thing` | G30 | heuristic | |
| `function-names-say-what-they-do` | G20 | heuristic | |
| `law-of-demeter` | G36 | heuristic | |
| `limited-public-api` | G8 | heuristic | |
| `long-names-for-long-scopes` | N5 | heuristic | |
| `max-arguments` | F1 | static | |
| `names-describe-side-effects` | N7 | heuristic | |
| `no-ambiguous-names` | N4 | heuristic | |
| `no-base-class-knowing-derivatives` | G7 | heuristic | |
| `no-clutter` | G12 | heuristic | fix |
| `no-commented-out-code` | C5 | static | |
| `no-dead-code` | G9 | static | fix |
| `no-dead-functions` | F4 | heuristic | |
| `no-duplication` | G5 | heuristic | |
| `no-encodings` | N6 | static | |
| `no-feature-envy` | G14 | heuristic | |
| `no-flag-arguments` | F3 | static | |
| `no-inappropriate-comments` | C1 | heuristic | |
| `no-inappropriate-statics` | G18 | heuristic | |
| `no-inherited-constants` | J2 | heuristic | |
| `no-magic-numbers` | G25 | static | |
| `no-multiple-languages` | G1 | heuristic | |
| `no-negative-conditionals` | G29 | static | fix |
| `no-obscured-intent` | G16 | heuristic | |
| `no-output-arguments` | F2 | heuristic | |
| `no-overridden-safeties` | G4 | static | |
| `no-pass-null` | Ch7 | static | |
| `no-redundant-comments` | C3 | heuristic | |
| `no-return-null` | Ch7 | static | |
| `no-selector-arguments` | G15 | heuristic | |
| `no-skipped-tests` | T3 | static | |
| `no-todo-tests` | T4 | static | |
| `one-assertion-per-test` | Ch9 | heuristic | |
| `one-level-of-abstraction` | G34 | heuristic | |
| `prefer-exceptions-over-error-codes` | Ch3 | heuristic | |
| `prefer-namespace-imports` | J1 | heuristic | |
| `prefer-polymorphism` | G23 | heuristic | |
| `use-explanatory-variables` | G19 | heuristic | |

## Library

Prefer to keep the checker inside `cargo test`? The engine is a regular library:

```rust
// Cargo.toml: clean-code-lints = "0.2"
let report = clean_code_lints::analyze_paths(
    &["src".into()],
    &clean_code_lints::Config::preset(clean_code_lints::Preset::Recommended),
);
assert!(report.is_clean(), "{:?}", report.diagnostics);
```

## Rust adaptations

The rule set started life as a JavaScript/TypeScript linter, so several rules are reinterpreted for Rust — `no-return-null` flags `Option<Vec<T>>` returns, `no-output-arguments` flags `&mut` parameters, `no-overridden-safeties` flags `#[allow]`/`unsafe`/`unwrap`. The full story is in [MAPPING.md](MAPPING.md).

## Development

```sh
cargo test                                      # 135 rule tests + 4 CLI integration tests
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo clean-code                                # the linter lints itself at strictest (clean-code.toml)
```

The repository lints itself with every rule at error (`preset = "strictest"` in `clean-code.toml`, enforced in CI) — the same self-linting discipline as the ESLint plugin.

## License

MIT
