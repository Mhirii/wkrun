# ADR 0001-005: Commit and Local Fast Gate

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Agents and developers need a canonical gate that is fast enough to run after ordinary implementation changes while still catching formatting, lint, compile, unit-test, and spelling problems.

A later repository-task-runner decision standardizes `just` as the canonical human/agent interface for composed repository operations. This ADR therefore uses `just check` as the stable entry point while retaining the previously agreed underlying checks.

## Decision

The canonical commit/local fast gate is:

```bash
just check
```

The `check` recipe performs the agreed baseline fast checks:

```bash
cargo fmt --all --check

cargo clippy \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  -- -D warnings

cargo test \
  --workspace \
  --lib \
  --bins \
  --all-features \
  --locked

typos
```

The `justfile` may invoke these commands directly while the recipe remains simple. If the operation later requires substantial shell logic, branching, temporary-directory management, artifact parsing, or similar complexity, that logic should move into a dedicated script/program invoked by the recipe.

### Test scope

The commit/local gate must remain fast and hermetic.

Fast unit tests belong here.

Tests requiring any of the following belong in later gates:

- Docker
- daemon orchestration
- subprocess-heavy fixtures
- external services
- timing-sensitive scenarios
- expensive integration machinery

### Validation-only behavior

The gate validates only.

It must not automatically rewrite source files.

Examples of operations that do **not** belong inside the gate:

- automatic `cargo fmt` modification
- `cargo clippy --fix`
- automatic typo rewriting

Fix commands may be run separately.

### Authority

Git hooks may invoke `just check`, but hooks remain optional.

Repository-owned commands and CI are authoritative.

Agents must run the commit/local gate before declaring ordinary implementation work complete unless the task explicitly cannot yet reach a compilable state.

## Consequences

### Positive

- One repeatable local command represents the baseline.
- Agents and humans run the same project-owned entry point.
- Integration-heavy tests do not make every edit expensive.
- The repository does not need a shell-script task runner for simple command composition.

### Trade-offs

- Contributors need `just` available to use the canonical convenience entry point.
- The local gate is intentionally not exhaustive.
- Passing this gate does not imply PR readiness.

## Related decision

Repository task-runner policy and `justfile` structure are defined in ADR 0001-017.
