# Repository Guidelines

## Project Structure & Module Organization

This crate is a Rust 2021 library for loading and querying npm configuration, ported from [@npmcli/config](https://github.com/npm/cli/tree/config-v11.0.1/workspaces/config). Public exports live in `src/lib.rs`.

Module layout tracks the upstream package so a contributor who knows the JavaScript version can navigate this one. Every source module has exactly one test file named after it:

| Rust source | Rust tests | Upstream origin |
|---|---|---|
| `src/nerf_dart.rs` | `tests/nerf_dart_tests.rs` | `lib/nerf-dart.js`, `test/nerf-dart.js` |
| `src/env_replace.rs` | `tests/env_replace_tests.rs` | `lib/env-replace.js`, `test/env-replace.js` |
| `src/error.rs` | (covered via `config_tests.rs`) | `lib/errors.js` |
| `src/config.rs` | `tests/config_tests.rs` | `lib/index.js` (loading, levels, merge) |
| `src/auth.rs` | `tests/auth_tests.rs` | `lib/index.js` (`getCredentialsByURI`) |
| `src/paths.rs` | `tests/paths_tests.rs` | `lib/index.js` (`loadGlobalPrefix`, local prefix) |
| `src/parser.rs` | `tests/parser_tests.rs` | `ini` parsing, `lib/parse-field.js` |
| `src/registry.rs` | `tests/registry_tests.rs` | `lib/definitions/definitions.js` (registry keys) |

`tests/integration_tests.rs` is the one cross-module file, covering end-to-end workflows with no single upstream counterpart.

When porting a behavior, name the Rust test after the upstream case (fixture names such as `nerfed_authToken` are kept verbatim) and reference the upstream file in the module doc comment. Keep focused unit tests beside their modules for crate-private helpers; put public API and filesystem behavior in `tests/*_tests.rs`. New modules must arrive with their matching `tests/<module>_tests.rs`. User-facing references belong in `docs/`; update `README.md` when public usage changes, and `docs/COMPATIBILITY.md` when parity with upstream changes. Do not commit generated `target/` contents.

## Build, Test, and Development Commands

- `cargo build` — compile the library with the default feature set.
- `cargo test --all-features` — run unit, integration, and documentation tests as CI does.
- `cargo fmt --all -- --check` — verify rustfmt output.
- `cargo clippy --all-targets --all-features -- -D warnings` — reject lint warnings.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` — validate API documentation.
- `cargo +1.83.0 check --all-features` — confirm the minimum supported Rust version.

Run formatting, Clippy, and tests before opening a pull request.

## Coding Style & Naming Conventions

Use rustfmt defaults (four-space indentation) and idiomatic Rust naming: `snake_case` for modules, functions, and tests; `PascalCase` for types and enum variants; `SCREAMING_SNAKE_CASE` for constants. Prefer small modules and existing standard-library or crate utilities over new abstractions. Document public APIs with `///`, including security-sensitive behavior and useful examples.

## Testing Guidelines

Use Rust’s built-in `#[test]` framework and `tempfile` for isolated filesystem cases. Name tests by behavior, following the existing `test_<behavior>` pattern. Add a regression test for parser, path, registry, or credential changes. There is no numeric coverage threshold; CI requires the complete test suite to pass on stable and beta Rust across Linux, macOS, and Windows.

## Commit & Pull Request Guidelines

History favors short imperative subjects, optionally with `feat:` or `fix:` (for example, `feat: load from file`). Keep commits focused. Pull requests should explain the behavior change, list validation commands, link relevant issues, and update tests and documentation when public behavior changes. Never include real registry tokens, passwords, certificates, or private `.npmrc` content in code, fixtures, logs, or screenshots.
