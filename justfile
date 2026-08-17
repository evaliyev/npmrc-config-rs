# List available commands.
default:
    @just --list

# Install optional development tools.
install-system-dependencies:
    cargo +stable install cargo-edit
    cargo +stable install cargo-llvm-cov
    cargo +stable install cargo-machete
    cargo +stable install cargo-watch

# Format Rust source.
format:
    cargo fmt --all

# Run all fast, non-mutating checks.
lint: check-cargo check-formatting check-clippy

# Run the complete pre-PR validation suite.
check: lint test docs msrv upstream-check

# Detect drift from the pinned @npmcli/config version.
upstream-check:
    node upstream/check.mjs

# Detect drift against the pinned version only, skipping the newer-tag probe.
# Upstream sources still need to be fetched once; they are cached per tag after that.
upstream-check-pinned:
    node upstream/check.mjs --no-tag-probe

# Regenerate upstream-derived case tables at the pinned version.
upstream-sync:
    node upstream/sync.mjs

# Repin to the newest config-v* tag, then regenerate.
upstream-sync-bump:
    node upstream/sync.mjs --bump

# Check compilation using the lockfile.
check-cargo:
    cargo check --locked --all-features

# Check rustfmt output.
check-formatting:
    cargo fmt --all -- --check

# Reject Clippy warnings.
check-clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests.
test:
    cargo test --all-features

# Reject rustdoc warnings.
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Check the minimum supported Rust version.
msrv:
    cargo +1.83.0 check --all-features

# Generate an HTML coverage report.
coverage:
    cargo llvm-cov test --all-features --html

# Serve the coverage report at http://localhost:7357.
serve-coverage:
    python3 -m http.server 7357 --directory target/llvm-cov/html

# Re-run tests when files change.
watch:
    cargo watch --clear --exec "test --all-features"

# Look for unused dependencies.
check-unused-dependencies:
    cargo machete

# Look for outdated dependencies.
check-for-updates:
    cargo upgrade --dry-run

# Update Cargo.toml and Cargo.lock dependencies.
update-dependencies:
    cargo upgrade
    cargo update
