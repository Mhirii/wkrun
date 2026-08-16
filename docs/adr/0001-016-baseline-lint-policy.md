# ADR 0001-016: Baseline Rust and Clippy Lint Policy

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

`cargo clippy -- -D warnings` provides a strong default baseline, but several project-specific invariants should also be machine-enforced.

The project wants strict correctness and safety without adopting broad opinionated lint groups that can create mechanical churn or low-signal suppressions.

## Decision

Repository lint policy is declared centrally in `Cargo.toml`.

The baseline is:

```toml
[lints.rust]
unsafe_code = "deny"
unsafe_op_in_unsafe_fn = "deny"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
```

CI continues to run Clippy with warnings denied:

```bash
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

### Unsafe policy

Owned `unsafe` code is denied by default, but not permanently forbidden.

If a future implementation genuinely requires `unsafe`, the exception must be:

- driven by a concrete requirement
- localized to the smallest practical boundary
- explicitly justified
- accompanied by documented safety invariants
- validated according to the deep-analysis/Miri policy where applicable

Use `deny`, not a repository-wide `forbid`, so a future narrowly justified exception remains possible through an explicit local decision.

### Panic and shortcut policy

`unwrap()`, `expect()`, and `panic!` are denied by default for ordinary application code.

A locally proven programmer invariant may use a narrow lint exception with an explicit reason.

Operational failures must continue to use the error-handling architecture rather than panic shortcuts.

`todo!()` and `unimplemented!()` must not remain in accepted production code.

### Debug artifacts

`dbg!()` is denied so accidental debugging output cannot silently land in production.

### stdout/stderr

Do **not** globally deny stdout/stderr printing.

Legitimate CLI presentation requires terminal output.

Internal diagnostics remain the responsibility of the `tracing` observability system.

### Broad lint groups

Do **not** globally enable:

- `clippy::pedantic`
- `clippy::nursery`
- `clippy::restriction`
- `clippy::cargo`

Individual lints from those groups may be adopted later when they demonstrate consistent value for `wkrun`.

### Documentation linting

Do **not** enable `missing_docs` globally at this stage.

Documentation requirements may be strengthened later at a meaningful public/reusable API boundary.

### Suppressions

Lint suppressions must be as local as practical.

When the rationale is not self-evident, the suppression must include a reason.

Agents must first determine whether a lint exposes a real defect.

Suppression is not the default response to a failing lint.

Broad crate/module-level suppressions used merely to make CI green are not acceptable.

## Consequences

### Positive

- Panic-prone shortcuts and debug debris are machine-enforced.
- Safe Rust is the default.
- Future legitimate unsafe work remains possible through explicit localized review.
- The project avoids turning broad subjective lint groups into mechanical style work.

### Trade-offs

- Some locally proven invariants require explicit lint exceptions.
- Pedantic/style policy remains intentionally selective rather than exhaustive.
