# ADR 0001-001: Initial Rust Architecture

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16
- **Scope:** Initial repository/crate/module structure only

## Context

Issue #1 establishes the Rust project foundation without implementing configuration, daemon behavior, runtime execution, Docker, Compose, or the TUI.

The PRD defines the intended architectural areas as:

- config/discovery
- project/workspace/service domain model
- daemon
- IPC
- runtime adapters
- CLI
- TUI

The roadmap also requires platform helpers plus shared error and tracing infrastructure.

The issue originally asked for an “appropriate Rust workspace, crate, and module structure without prematurely splitting into micro-crates.” That wording leaves too much discretion to an implementation agent.

## Decision

`wkrun` starts as a **single application crate**.

The intended top-level architectural modules are:

- `cli`
- `config`
- `domain`
- `daemon`
- `ipc`
- `runtime`
- `tui`
- `platform`

Shared error and tracing infrastructure are also part of the architecture.

Only modules needed by the current issue should be created immediately. The remaining modules should be introduced when their implementation begins rather than as empty placeholders.

`main.rs` must remain limited to application bootstrap and top-level wiring. It must not become the implementation location for future subsystems.

The project must not be split into multiple internal crates at this stage.

A competing top-level architecture must not be introduced without concrete justification.

Abstractions, traits, or crate boundaries must not be added solely for hypothetical future extensibility.

## Consequences

### Positive

- The architecture is explicit without freezing an unnecessary file tree.
- Agents have enough freedom to make normal Rust module-level implementation choices.
- Empty placeholder modules are avoided.
- Future crate splitting can be driven by demonstrated implementation pressure rather than speculation.
- `main.rs` remains a composition boundary instead of becoming a monolith.

### Trade-offs

- Some module boundaries will only become concrete when their implementations begin.
- A future decision may split one or more modules into crates if there is a demonstrated reason.

## Non-decisions

This ADR does **not** decide:

- the exact file tree beneath each module
- whether any specific module will become a separate crate later
- the CLI parsing library
- daemon/runtime/TUI implementation details
