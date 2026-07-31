# Repository Guidelines

## Project Structure & Module Organization

This crate is a Rust 2021 library for loading and querying npm configuration. Public exports live in `src/lib.rs`; implementation is split by responsibility:

- `src/config.rs` loads and merges configuration levels.
- `src/parser.rs` parses values and expands environment variables.
- `src/auth.rs` and `src/registry.rs` handle credential scoping and registry lookup.
- `src/paths.rs` resolves npm configuration locations; `src/error.rs` defines errors.

Keep focused unit tests beside their modules. Put cross-module and filesystem behavior in `tests/*_tests.rs`. User-facing references belong in `docs/`; update `README.md` when public usage changes. Do not commit generated `target/` contents.

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
