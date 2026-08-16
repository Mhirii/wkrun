# ADR 0001-007: Dependency and Supply-Chain Baseline

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

`wkrun` will eventually receive environment snapshots, execute local processes, interact with Docker/Compose, access files, use Unix signals/process groups, and maintain long-running runtime state.

Dependencies therefore participate in a tool that users entrust with meaningful capability on their machines.

The initial automated policy is enforced primarily through `cargo-deny` and the committed dependency graph.

## Decision

### Cargo-deny

The repository commits a `deny.toml`.

`cargo deny check` is required in PR CI.

### Advisory policy

The following fail CI:

- known security vulnerabilities
- unsoundness advisories
- yanked dependencies

Unmaintained dependencies initially produce a visible warning rather than an automatic failure.

Advisory ignores must be explicit and justified.

Risk-bearing exceptions should reference a tracking issue when they are not immediately resolved.

### Dependency sources

crates.io is the default trusted public registry.

Unknown registries are denied.

Unknown Git sources are denied.

Git dependencies are prohibited by default.

Any explicitly approved Git dependency must:

- be allowlisted
- be pinned to an immutable revision

Floating branch-based Git dependencies are not accepted as the normal policy.

### Version policy

Wildcard dependency versions are denied.

### Duplicate dependencies

Multiple versions of the same crate are denied by default.

Legitimately unavoidable duplicates require narrow documented exceptions.

Agents must not distort dependency selection or force inappropriate upgrades solely to eliminate a justified duplicate.

### License allowlist

The initial permissive license allowlist is:

- MIT
- Apache-2.0
- Apache-2.0 WITH LLVM-exception
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Zlib
- Unicode-3.0

Licenses outside this allowlist require explicit review before being permitted.

### Exceptions

Exceptions must be narrow and include a reason.

Actionable dependency/security debt should have a tracking issue.

### Trust boundary

Passing dependency-policy tooling is **not** considered proof that dependency code itself is trustworthy.

`cargo-deny` provides automated policy and known-risk enforcement; code-trust auditing is a separate concern.

## Consequences

### Positive

- Dependency sources and versions are intentional.
- Known advisories and yanked dependencies fail closed.
- License policy is explicit rather than accidental.
- Duplicate dependency cost is visible and controlled.

### Trade-offs

- Some legitimate dependencies may require explicit policy exceptions.
- Unmaintained crates are initially warnings rather than hard failures.
- This baseline does not by itself audit dependency source code.
