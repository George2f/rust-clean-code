# clean-code-lints

[![CI](https://github.com/george2f/rust-clean-code/actions/workflows/ci.yml/badge.svg)](https://github.com/george2f/rust-clean-code/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/clean-code-lints.svg)](https://crates.io/crates/clean-code-lints)
[![docs.rs](https://img.shields.io/docsrs/clean-code-lints)](https://docs.rs/clean-code-lints)
[![license](https://img.shields.io/crates/l/clean-code-lints.svg)](LICENSE)

Clean Code lints for Rust, derived from the
[clean-code ESLint plugin](https://github.com/george2f/eslint-plugin-clean-code). All 44 rules —
same names, same Clean Code identifiers, same 12 static / 32 heuristic split, same
`recommended` / `strict` / `strictest` presets — implemented for Rust with [`syn`].

Runs on stable Rust. No compiler internals, no nightly, no `rustc_private`.

## Usage

```sh
cargo install cargo-clean-code
cargo clean-code                       # recommended preset, current directory
cargo clean-code src tests --preset strict
cargo clean-code --config clean-code.toml --fix
cargo clean-code --format json | jq .  # machine-readable output
```

Exit codes: `0` clean (or warnings only), `1` errors or files that failed to parse, `2` invalid
configuration or flags.

Rules can be adjusted from the command line:

```sh
cargo clean-code --deny no-magic-numbers --allow law-of-demeter --warn no-todo-tests
```

## Configuration

`clean-code.toml` in the project root is picked up automatically:

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

## Rules

The full rule catalog with Rust-specific implementation notes lives in [MAPPING.md](MAPPING.md).

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

The lint engine is also a library, useful in a project's own test suite:

```rust
// Cargo.toml: clean-code-lints = "0.1"
let report = clean_code_lints::analyze_paths(
    &["src".into()],
    &clean_code_lints::Config::preset(clean_code_lints::Preset::Recommended),
);
assert!(report.is_clean(), "{:?}", report.diagnostics);
```

## Rust adaptations

The ESLint plugin targets JavaScript/TypeScript; several rules are reinterpreted for Rust (for
example `no-return-null` flags `Option<Vec<T>>` returns, `no-output-arguments` flags `&mut`
parameters, `no-overridden-safeties` flags `#[allow]`/`unsafe`/`unwrap`). Every rule, its Rust
semantics and its known approximations are documented in [MAPPING.md](MAPPING.md).

## Development

```sh
cargo test                                      # 135 rule tests + 4 CLI integration tests
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo clean-code                                # the linter lints itself at strictest (clean-code.toml)
```

The repository lints itself with every rule at error (`preset = "strictest"` in `clean-code.toml`,
enforced in CI), the same self-linting discipline as the ESLint plugin.

## License

MIT
