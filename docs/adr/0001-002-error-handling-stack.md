# ADR 0001-002: Error Handling Stack and Boundaries

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

`wkrun` will eventually report errors involving configuration, projects/workspaces, services, runtime failures, IPC, Docker/Compose, and platform behavior.

Errors need to remain structured enough for callers to distinguish meaningful failure kinds while preserving original causes. User-facing CLI/TUI presentation must remain separate from internal error representation.

## Decision

`wkrun` standardizes on:

```toml
[dependencies]
anyhow = "1"
thiserror = "2"
```

### Typed subsystem errors

Use `thiserror` for typed subsystem/application errors.

Subsystem APIs should return concrete typed error types when callers may need to:

- distinguish failure kinds
- retain domain context
- inspect or match the error programmatically

Underlying causes must be preserved with `#[source]` and/or `#[from]` where appropriate.

Stable failure categories should not be represented only as arbitrary strings when a meaningful typed variant exists.

### Application-boundary errors

Use `anyhow` only at application boundaries where heterogeneous failures need to be propagated or enriched with operation-level context.

`anyhow::Result` must **not** become the default return type throughout:

- `config`
- `domain`
- `daemon`
- `ipc`
- `runtime`
- `platform`

Use `anyhow::Context` / `with_context` to add useful execution context without discarding the underlying cause.

### Presentation boundary

Internal error types must not be designed around CLI formatting.

The CLI/TUI boundary is responsible for converting internal errors into concise, actionable user-facing reports.

### Panic policy

`panic!`, `unwrap()`, and `expect()` are not error-handling mechanisms for ordinary recoverable failures.

`unwrap()` / `expect()` are acceptable only when a local invariant has already proven that failure would indicate a programmer bug rather than:

- user input
- configuration
- runtime conditions
- external resource state
- other expected operational failure

### Competing frameworks

Do not introduce another general error/reporting framework such as:

- `eyre`
- `color-eyre`
- `miette`

without an explicit architectural decision.

Do not use `Box<dyn Error>` as the project's general substitute for typed errors.

## Intended flow

```text
subsystem
  -> typed thiserror error
  -> preserve source/context
  -> application boundary may use anyhow
  -> CLI/TUI renders user-facing report
```

## Consequences

### Positive

- Subsystems retain meaningful error semantics.
- Error chains remain inspectable.
- Top-level orchestration remains ergonomic.
- Presentation stays decoupled from domain/runtime error design.

### Trade-offs

- Authors must decide whether a boundary needs a concrete typed error or application-level aggregation.
- Some boilerplate is accepted in exchange for stronger semantics.
