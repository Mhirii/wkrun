# ADR 0001-009: Merge Authoritative Gate

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Two pull requests may independently pass against an earlier `main` while their combined integration result fails.

A green PR therefore does not prove that the exact candidate entering `main` is valid.

## Decision

`main` is protected.

Normal development reaches `main` only through the validated PR/merge path.

The exact integration candidate that will enter `main` must pass the merge gate.

A previously green PR is insufficient when:

- the candidate changed
- the base advanced
- conflict resolution changed content
- a rebase modified content
- the lockfile changed
- any other amendment changed the candidate

Previous validation is tied to the candidate being tested.

### Linux and macOS merge checks

Both platforms run:

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

### Once per candidate

Run:

```bash
cargo deny check

cargo machete

RUSTDOCFLAGS="-D warnings" \
  cargo doc \
  --workspace \
  --all-features \
  --no-deps \
  --locked
```

Once Cargo-vet becomes mandatory before Phase 3, also run:

```bash
cargo vet
```

### Checks not normally repeated

The following need not normally rerun at merge time if the exact relevant content already passed PR validation and has not changed:

- `rustfmt`
- `typos`
- `actionlint`
- `zizmor`

### Flakes

Required merge checks do not use automatic retries to hide flaky tests.

### Integration candidate availability

If the exact integration candidate cannot be validated directly, the PR must be updated against current `main` and the required merge checks rerun before merging.

### Direct pushes

Direct pushes to `main` are break-glass exceptions only, not normal development workflow.

## Consequences

### Positive

- The commit entering `main` has current integration evidence.
- Concurrent PRs cannot rely indefinitely on stale validation.
- Merge latency is kept lower by avoiding unnecessary repetition of content-local checks.

### Trade-offs

- Base-branch changes may require additional CI before merge.
- Protected-branch configuration becomes part of repository operations.
