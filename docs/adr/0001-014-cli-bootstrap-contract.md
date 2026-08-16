# ADR 0001-014: CLI Bootstrap Contract

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Issue #1 must create a real executable that exposes basic help and version information without prematurely introducing nonfunctional MVP commands.

The CLI parser is foundational API surface: argument parsing, help behavior, version behavior, and bare invocation semantics should not be left to implementation-agent preference.

## Decision

### CLI parsing library

`wkrun` standardizes on:

```toml
clap = { version = "4", features = ["derive"] }
```

Use Clap's derive API as the default declaration style.

The builder API may be used later when the derive API becomes materially awkward, but implementations should not mix styles without a concrete reason.

Do not introduce a competing CLI parsing framework without an explicit architectural decision.

### Issue #1 command surface

Issue #1 implements only the root CLI parser.

Do **not** create future MVP subcommands such as `up`, `logs`, `restart`, or `tui` as nonfunctional placeholders.

Future commands should be added when their behavior is implemented.

### Help

Both:

```bash
wkrun --help
wkrun -h
```

must:

- print the root help
- exit with status `0`

### Version

Both:

```bash
wkrun --version
wkrun -V
```

must:

- print version information
- exit with status `0`

The reported version must come from Cargo package metadata rather than being manually duplicated in source.

### Bare invocation

Running:

```bash
wkrun
```

with no arguments prints the root help and exits with status `0`.

Bare `wkrun` must never implicitly perform a state-changing operation.

This is a stable UX principle: after subcommands are added later, bare `wkrun` continues to show root help rather than becoming an implicit default such as `wkrun up`.

### Invalid input

Unknown arguments or subcommands use normal Clap parse-error behavior and exit non-zero.

### Architectural boundary

CLI parsing must remain separate from application/business logic.

Clap-derived types represent parsed user intent; they must not become the location for domain, daemon, runtime, or orchestration behavior.

## Consequences

### Positive

- The bootstrap binary has deterministic behavior.
- The first-run experience is useful rather than erroring on an empty invocation.
- Future state-changing commands remain explicit.
- Version metadata has a single source of truth.
- Placeholder command surface does not become accidental API commitment.

### Trade-offs

- The initial CLI is intentionally minimal.
- Future command issues must extend the parser as behavior becomes real.

## Non-decisions

This ADR does **not** define:

- future MVP subcommand argument shapes
- shell completion behavior
- CLI output styling beyond the bootstrap help/version contract
- runtime/daemon interactions
