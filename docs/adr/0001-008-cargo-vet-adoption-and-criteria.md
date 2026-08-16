# ADR 0001-008: Cargo-vet Adoption Timing and Audit Criteria

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

`cargo-deny` covers policy and known-risk checks but does not establish that dependency code has a meaningful audit/trust path.

`cargo-vet` provides that separate code-trust layer.

However, requiring a complete trust database during the bootstrap issue would front-load manual process before the dependency graph has stabilized.

The risk changes materially when `wkrun` begins executing and supervising user processes.

## Decision

### Adoption timing

`cargo-vet` is **not mandatory from Issue #1**.

The foundation phase uses the dependency controls defined in ADR 0001-007.

`cargo-vet` must be initialized and brought to a meaningful trust baseline **before Phase 3 — Process Runtime**.

From Phase 3 onward:

- `cargo vet` is a mandatory PR gate
- `cargo vet` is also part of the merge candidate validation

### Required criteria

Once enabled:

```text
normal/runtime dependencies -> safe-to-deploy
build dependencies          -> safe-to-deploy
dev/test-only dependencies  -> safe-to-run
```

`safe-to-deploy` satisfies `safe-to-run`.

Build dependencies, including procedural macros and build-time code generation dependencies, retain the stronger `safe-to-deploy` requirement.

### Criteria policy

Use Cargo-vet's standardized built-in criteria rather than creating project-specific criteria without a concrete unmet security requirement.

Custom criteria may be introduced later only for narrowly defined assurance properties not adequately represented by the built-in criteria.

### Audit reuse and upgrades

Existing trusted audit paths should be reused where appropriate.

Delta audits should be preferred for dependency upgrades when they provide an appropriate review path.

Dependency upgrades must preserve a valid audit path.

### Exemptions

Cargo-vet exemptions represent temporary audit debt, not approval.

New exemptions require:

- an explicit reason
- a tracking issue when the debt is not immediately resolved

Agents must not add or regenerate exemptions merely to make `cargo vet` pass.

The desired direction is to reduce exemptions over time rather than accumulate them.

## Security principle

`wkrun` is privileged by trust, not necessarily by OS privilege.

Capabilities such as:

- process execution
- environment access
- filesystem access
- Docker access
- signals
- persisted runtime identity

must be treated as security-sensitive.

Security controls should respond to concrete threats rather than becoming checkbox compliance.

## Consequences

### Positive

- Manual dependency trust work begins before high-authority runtime behavior.
- Bootstrap work is not burdened with premature audit ceremony.
- Runtime/build dependencies receive a stronger trust criterion than dev-only tooling.
- Audit debt remains explicit.

### Trade-offs

- The project has a defined future gate activation point that must not be missed.
- Cargo-vet introduces human review and trust-management work once enabled.
