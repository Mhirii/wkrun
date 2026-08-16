# ADR 0001-006: Pull Request Comprehensive Gate

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Pull requests are the first stage where `wkrun` intentionally pays substantial CI cost for full correctness, dependency policy, documentation, workflow validation, and both MVP operating systems.

## Decision

Every pull request must pass a required comprehensive suite.

### Linux and macOS platform matrix

Both Linux and macOS independently run:

```bash
cargo clippy \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  -- -D warnings

cargo nextest run \
  --workspace \
  --all-features \
  --locked \
  --profile ci

cargo test \
  --doc \
  --workspace \
  --all-features \
  --locked
```

Both platform jobs are mandatory.

Platform-specific failures must not be configured as allowed-to-fail.

### Nextest policy

`.config/nextest.toml` is repository-owned.

CI uses zero automatic test retries.

Flaky tests are defects and must not be hidden by retry-until-green behavior.

### Platform-independent checks

Run once per PR:

```bash
cargo fmt --all --check

typos

cargo deny check

cargo machete

RUSTDOCFLAGS="-D warnings" \
  cargo doc \
  --workspace \
  --all-features \
  --no-deps \
  --locked

actionlint

zizmor .
```

Platform-independent checks run once to avoid unnecessary duplicated CI cost.

### Exceptions

Policy/configuration exceptions for tools such as:

- `cargo-deny`
- `cargo-machete`
- `actionlint`
- `zizmor`

must be:

- explicit
- narrow
- committed
- justified

The tool must not simply be bypassed.

### Read-only validation

PR validation must not automatically modify source or repository configuration.

### Separate build job

A blanket standalone `cargo build` PR job is not required unless a future build path is discovered that is not meaningfully exercised by Clippy/tests/docs.

## Consequences

### Positive

- Both supported MVP platforms are treated as real product targets.
- Test flakes become visible immediately.
- Dependency, docs, and CI-workflow quality are enforced before merge.
- Platform-independent checks do not waste matrix capacity.

### Trade-offs

- PR CI is substantially heavier than the local gate.
- New tools require repository-owned configuration and justified exceptions.
