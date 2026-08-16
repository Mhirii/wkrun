set shell := ["bash", "-cu"]

# Default recipe lists available recipes.
default:
    @just --list

# Build the workspace in debug mode.
build:
    cargo build --workspace --locked

# Verify formatting (no rewrite).
fmt-check:
    cargo fmt --all --check

# Run the strict linter.
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Run the library, binary, and integration tests.
test:
    cargo test --workspace --lib --bins --all-features --locked

# Run the cargo-nextest ci profile.
nextest:
    cargo nextest run --workspace --all-features --locked --profile ci --no-tests=warn

# Run typos (validation only).
typos:
    typos

# Run cargo-deny against the committed policy.
deny:
    cargo deny check

# Run cargo-machete against the committed dependencies.
machete:
    cargo machete

# Build docs with warnings denied.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked

# Verify the justfile itself is canonically formatted.
justfile-check:
    just --fmt --check

# Canonical local fast gate.
check: fmt-check justfile-check clippy test typos
    @echo "all checks passed"
