# ADR 0001-015: Rust Toolchain and Package Metadata

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Compiler, Cargo, rustfmt, and Clippy versions can change observable build and lint behavior.

The repository therefore needs a deterministic development/CI toolchain without accidentally declaring a minimum-supported-Rust-version compatibility promise that has not been discussed.

The project is currently a single application crate and should not introduce a virtual workspace solely for future possibilities.

## Decision

### Rust edition

Use Rust edition:

```toml
edition = "2024"
```

### Repository-pinned toolchain

Commit `rust-toolchain.toml`.

Initially:

```toml
[toolchain]
channel = "1.97.1"
profile = "minimal"
components = ["rustfmt", "clippy"]
```

The repository pins an **exact stable Rust release** rather than the floating `stable` channel.

Local development and CI use the repository-pinned toolchain.

Toolchain upgrades are explicit repository changes and must pass the normal PR/merge quality gates.

### MSRV

Do **not** declare `package.rust-version` yet.

The project makes no minimum-supported-Rust-version compatibility promise during early development.

The repository-pinned development/build toolchain and a future MSRV promise are distinct concepts.

### Workspace/package layout

Keep the root package as the workspace root.

Do not create a virtual workspace or move the application into `crates/wkrun/` merely to anticipate future crate splitting.

Declare Rust 2024 resolver semantics explicitly:

```toml
[workspace]
resolver = "3"
```

### Package metadata

Initial package metadata includes:

```toml
[package]
name = "wkrun"
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/mhirii/wkrun"
publish = false
```

### Lockfile

Commit `Cargo.lock`.

Cargo operations in the agreed quality gates use `--locked` where dependency resolution applies.

### crates.io publication

Set:

```toml
publish = false
```

to prevent accidental crates.io publication.

Publishing `wkrun` to crates.io, including enabling `cargo install wkrun` from crates.io, requires a future explicit decision.

## Consequences

### Positive

- Local and CI compiler/lint behavior is reproducible.
- Compiler upgrades are reviewable engineering changes.
- The project does not accidentally promise compatibility with older compilers.
- The single-crate architecture remains simple.
- Accidental crates.io publication is prevented.

### Trade-offs

- The pinned toolchain requires periodic deliberate updates.
- Contributors use the repository-selected compiler rather than whatever their machine currently calls `stable`.
