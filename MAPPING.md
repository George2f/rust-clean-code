# Rust mapping

All 44 rules of the [clean-code ESLint plugin](https://github.com/george2f/eslint-plugin-clean-code)
are ported to Rust with the same rule names, Clean Code identifiers and tiers (12 static, 32
heuristic). This document records, per rule, what was ported as-is, what was adapted to Rust, and
where the implementation is an approximation.

## Rules

| Rule | Clean Code | Tier | Fix | Rust implementation |
| --- | --- | --- | --- | --- |
| `be-precise` | G26 | heuristic | | Float literal equality and `f32`/`f64` casts compared with `==`/`!=`; JS `parseInt` concerns do not apply. |
| `command-query-separation` | Ch3 | heuristic | | Methods with `&mut self` whose return type is not `()` are flagged. |
| `declaration-proximity` | G10 | heuristic | | A `let` whose first use is more than `maxDistance` (default 15) statements later is flagged. |
| `descriptive-names` | N1 | heuristic | | Single-letter names outside `allow`, plus a `forbidden` list of vague names (`data`, `tmp`, ...). |
| `encapsulate-boundaries` | G33 | heuristic | | Arithmetic (`+`/`-`) on `.len()` / `.count()` / `.capacity()` results is flagged. |
| `encapsulate-conditionals` | G28 | heuristic | | Conditions with more than `maxLogicalOperators` (default 2) `&&`/`||` operators are flagged. |
| `function-does-one-thing` | G30 | heuristic | | Functions over `maxStatements` (20) or `maxLines` (30) are flagged. |
| `function-names-say-what-they-do` | G20 | heuristic | | Function names containing a forbidden word as a `_`-separated segment. The default list drops the JS-isms `fn`/`func`: `_fn` suffixes (`visit_item_fn`, `record_fn`) are idiomatic Rust. |
| `law-of-demeter` | G36 | heuristic | | Method/field chains longer than `maxChainLength` (3). A default allow list of iterator and `Option`/`Result` combinators keeps idiomatic chains quiet. |
| `limited-public-api` | G8 | heuristic | | More than `maxExports` (50) `pub` items per file or `maxClassMembers` (25) `pub` methods per impl. |
| `long-names-for-long-scopes` | N5 | heuristic | | Module-level item names shorter than `minLength` (3), same as the plugin's module-scope check. |
| `max-arguments` | F1 | static | | More than `max` (3) parameters, receiver excluded. |
| `names-describe-side-effects` | N7 | heuristic | | Methods named `get_*`/`is_*`/`has_*`/`find_*` that take `&mut self` or a `&mut` parameter. |
| `no-ambiguous-names` | N4 | heuristic | | Names in `confusingNames` (`l`, `O`, `I`) and names differing only by case. |
| `no-base-class-knowing-derivatives` | G7 | heuristic | | Trait default bodies that reference a type implementing that trait. Path-based approximation: only implementors visible in the analyzed source are known. |
| `no-clutter` | G12 | heuristic | fix | `dbg!` macros are flagged and unwrapped by the fix. |
| `no-commented-out-code` | C5 | static | | Comments whose text parses as a Rust statement, expression or item. `//` inside string literals is ignored. |
| `no-dead-code` | G9 | static | fix | Unreachable statements after `return`/`break`/`continue`/`panic!`. Unused variables and imports are rustc's `dead_code`/`unused` territory and are not duplicated. |
| `no-dead-functions` | F4 | heuristic | | Project-wide reference counting; `pub`, trait impls, `#[test]`, `#[no_mangle]` and `main` are exempt. |
| `no-duplication` | G5 | heuristic | | Token-normalized function bodies (min `minStatements` 4, `minNodes` 25) hashed across the analyzed files. |
| `no-encodings` | N6 | static | | Type-word prefixes (`str_name`, `strName`) and extra `patterns`. |
| `no-feature-envy` | G14 | heuristic | | Methods accessing a parameter more than `minimumAccesses` (4) times and more than `self`. |
| `no-flag-arguments` | F3 | static | | `bool` parameters. |
| `no-inappropriate-comments` | C1 | heuristic | | History markers (`previously`, `used to`, ...) and ISO date stamps in comments. |
| `no-inappropriate-statics` | G18 | heuristic | | Unit structs with two or more associated functions and no receiver; use free functions or a module. |
| `no-inherited-constants` | J2 | heuristic | | `Type::CONST` paths where `CONST` is declared in a trait and `Type` is not that trait. |
| `no-magic-numbers` | G25 | static | | Numeric literals outside `const`/`static`; `ignore` defaults to `[-1, 0, 1, 2]` and enum discriminants are ignored. `#[cfg(test)]` modules are exempt so fixtures may use literals. |
| `no-multiple-languages` | G1 | heuristic | | String literals containing word-boundary SQL keyword pairs (`select`/`from`, `insert`/`into`, ...) or an HTML tag opener. `#[cfg(test)]` modules are exempt so fixtures may embed other languages. |
| `no-negative-conditionals` | G29 | static | fix | `if !cond { .. } else { .. }`; the fix swaps the branches and drops the negation. |
| `no-obscured-intent` | G16 | heuristic | | If-expressions nested as the tail of another branch (the nested-ternary analog); `else if` chains are allowed. |
| `no-output-arguments` | F2 | heuristic | | Non-receiver `&mut` parameters. |
| `no-overridden-safeties` | G4 | static | | `#[allow(...)]` attributes, `unsafe` blocks, `.unwrap()` and `.expect()` calls. |
| `no-pass-null` | Ch7 | static | | Literal `None` passed as a call argument; `ignoreCallees` entries match callee name prefixes. |
| `no-redundant-comments` | C3 | heuristic | | Comments with two or more significant words, all of which also appear on the next code line. |
| `no-return-null` | Ch7 | static | | `Option<Vec<T>>`, `Option<HashMap<..>>`, `Option<&[T]>` and similar collection returns; return an empty collection instead. Scalar `Option` and `Option<String>` are idiomatic and stay clean. |
| `no-selector-arguments` | G15 | heuristic | | Non-bool parameters used only in comparisons or as `match`/`if` scrutinees (`minComparisons`, default 2). Bool flags belong to `no-flag-arguments`. |
| `no-skipped-tests` | T3 | static | | `#[ignore]`. The JS `testFunctions` option is not applicable. |
| `no-todo-tests` | T4 | static | | `#[test]` functions that are empty or contain `todo!`/`unimplemented!`. |
| `one-assertion-per-test` | Ch9 | heuristic | | `#[test]` functions with more than `maxAssertions` (1) `assert*` macros. |
| `one-level-of-abstraction` | G34 | heuristic | | Rust low-level method list (`trim`, `split`, `parse`, `push`, ...) plus arithmetic with literals, against free-function and non-low-level method calls; closure bodies are skipped. |
| `prefer-exceptions-over-error-codes` | Ch3 | heuristic | | Functions returning `bool` whose body contains a `false` literal. |
| `prefer-namespace-imports` | J1 | heuristic | | Braced `use` lists with more than `maxNamedImports` (10) names. Note: `clippy::wildcard_imports` advises the opposite; pick one policy per project. |
| `prefer-polymorphism` | G23 | heuristic | | `match` with more than `maxSwitchCases` (4) literal/constant arms or if/else chains longer than `maxIfElseBranches` (5). Matches that destructure enum variants are the Rust form of polymorphism and are allowed. |
| `use-explanatory-variables` | G19 | heuristic | | Conditions containing indexing (`values[0]`) or method chains of two or more links. |

## Known approximations

These rules are path/AST-based and have no type information. Raise or silence them per project via
`clean-code.toml` if a codebase triggers false positives:

- `no-base-class-knowing-derivatives` and `no-inherited-constants` only see implementors and trait
  constants defined in the analyzed files.
- `no-feature-envy` matches parameter names, not resolved receiver types.
- `no-duplication` hashes the full function body; distinct bodies that differ in one identifier do
  not match.
- Comment rules (`no-commented-out-code`, `no-redundant-comments`, `no-inappropriate-comments`)
  work on source text with a quote-aware scanner, not the lexer.
- `no-magic-numbers` and `no-multiple-languages` deliberately skip `#[cfg(test)]` modules: test
  fixtures use literal numbers and embedded languages on purpose.
