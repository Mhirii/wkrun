# wkrun Agent Guide

`wkrun` is a Linux/macOS local development orchestrator for running and supervising the multiple services that make up a development project.

The core domain hierarchy is:

```text
Project -> Workspace -> Service
```

The MVP is config-driven and includes:

* local process services
* Docker services
* Docker Compose services
* dependencies
* readiness
* fixed and dynamic host ports
* lifecycle supervision
* logs
* CLI and TUI control through a single per-user daemon

Git worktree orchestration and automatic service detection are post-MVP directions.

## Authority

For issue-backed work, use this authority order:

1. Current explicit user instructions.
2. `docs/PRD.md` for product behavior and externally observable semantics.
3. The GitHub issue for current scope and acceptance criteria.
4. `ROADMAP.md` for sequencing and milestone boundaries.
5. Existing code and tests for current implementation behavior.

The PRD defines what `wkrun` means.

A GitHub issue may narrow scope or describe the work to perform, but it must not silently override contradictory PRD behavior. If an issue and the PRD conflict, surface the contradiction before implementing new behavior.

`ROADMAP.md` is a sequencing document, not a product-specification authority.

Do not modify `docs/PRD.md`, `ROADMAP.md`, or this file unless the task explicitly requires it.

## Product Decisions vs Implementation Decisions

Agents may make ordinary private implementation decisions when they do not alter externally observable behavior.

Examples include:

* internal module organization
* private helper APIs
* SQLite schema/index details
* internal ID representation
* test helpers
* private IPC encoding details that do not alter protocol compatibility or client-visible behavior
* internal data structures
* straightforward error-type organization

Agents must not silently make product decisions.

A decision is product-level when it changes externally observable behavior, including:

* CLI syntax or semantics
* config syntax or validation behavior
* lifecycle/state transitions
* desired-state behavior
* persistence semantics visible across invocations
* readiness behavior
* port behavior
* service/resource ownership
* daemon/client behavior
* Docker/Compose cleanup semantics
* TUI interaction semantics
* protocol compatibility or upgrade behavior

If implementation requires externally observable behavior that is not determined by the PRD or current instructions, report the exact ambiguity instead of guessing.

## Scope

* Implement only the requested issue and required prerequisites.
* Satisfy the issue acceptance criteria completely.
* Do not implement post-MVP features merely because current architecture anticipates them.
* Avoid unrelated refactors.
* Do not redesign adjacent working code without a concrete requirement.
* Avoid speculative abstractions for hypothetical future use.
* Do not weaken validation or tests to make an implementation pass.
* Do not silently change product semantics because another implementation would be easier.

Known post-MVP direction should influence boundaries, not expand current scope.

## Architectural Invariants

Do not violate these without an explicit product decision:

* Linux and macOS are the supported platforms.
* The domain hierarchy is `Project -> Workspace -> Service`.
* Core domain and runtime logic must remain independent of CLI and TUI presentation code.
* MVP uses one per-user daemon as the authority for live `wkrun` runtime state.
* CLI and TUI are daemon clients; they do not own service lifetimes.
* SQLite stores durable registry, intent, allocation, and historical metadata, but is never authoritative for current live runtime state.
* Desired state and observed runtime/health state are distinct concepts.
* A daemon crash must not intentionally terminate running services.
* Process/container/resource ownership must be positively established before destructive cleanup.
* Never treat a persisted PID alone as proof of process ownership or liveness.
* `wkrun` supervises outer development processes; it does not implement source watching or hot reload.
* Config-relative paths resolve from the directory containing the selected config file.
* Git repository/worktree roots are metadata and do not redefine config-relative paths.
* Config discovery must preserve the distinction between explicit wkrun-owned filenames and generic `project.*` files.
* Invalid explicit wkrun config files are errors; unrelated generic `project.*` files are ignored unless they positively validate as wkrun configuration.
* Lifecycle actions use the initiating client process's environment snapshot according to the PRD, never the daemon's stale startup environment.
* Runtime-specific automatic restart mechanisms must not fight `wkrun` lifecycle supervision for resources owned by `wkrun`.
* Compose and Docker cleanup must respect ownership boundaries.

Read `docs/PRD.md` for detailed lifecycle, readiness, daemon, port, Compose, and CLI semantics rather than duplicating them here.

## Working on an Issue

Before editing:

1. Read the GitHub issue and all acceptance criteria.
2. Read the relevant sections of `docs/PRD.md`.
3. Inspect the affected implementation and existing tests.
4. Check callers, state ownership, and cleanup paths before changing shared behavior.
5. Identify whether the work requires any externally observable behavior not already specified.

Do not implement an issue from its title alone.

For architectural or cross-cutting work, inspect enough surrounding code to understand:

* ownership
* state transitions
* persistence boundaries
* failure paths
* cleanup behavior
* daemon/client responsibilities

If a product-level ambiguity exists, surface it before encoding an assumption.

## Configuration Work

When working on configuration:

* TOML and YAML map to the same logical schema.
* Keep parsing, structural validation, semantic validation, discovery, and interpolation as distinct concerns where practical.
* Do not make the interpolation layer allocate ports or implicitly read daemon-global environment state.
* Unknown/invalid user configuration should fail with actionable context rather than being silently ignored, except where the PRD explicitly defines generic `project.*` discovery behavior.
* Config discovery and config-relative path resolution must remain deterministic.

## Runtime and Lifecycle Work

When touching process, daemon, Docker, Compose, dependency, or readiness behavior:

* preserve the distinction between desired state and observed state
* consider intentional stop separately from unexpected exit
* consider daemon crash/recovery
* consider repeated/idempotent operations
* consider stale persisted state
* consider ownership before cleanup
* consider partially started resources
* consider dependency recovery
* avoid creating competing lifecycle authorities

For process services, ensure child process trees are not accidentally orphaned.

For Docker/Compose services, do not destroy external/manual resources merely because they resemble `wkrun` resources.

## Testing

Tests should prove issue acceptance criteria and observable behavior.

For `wkrun`, pay particular attention to:

* desired-state vs observed-state transitions
* `blocked`, `degraded`, `unhealthy`, `failed`, and recovery transitions
* dependency startup and recovery
* daemon crash/reconciliation paths
* process/container ownership
* cleanup behavior
* repeated and idempotent lifecycle operations
* stale persisted state
* concurrent daemon startup
* concurrent/global port allocation
* fixed-port failure
* readiness timeout and recovery
* crash restart counters
* process-tree cleanup
* Compose partial-start and ownership behavior

Prefer deterministic synchronization over arbitrary sleeps in runtime/concurrency tests.

Use real temporary filesystem trees where filesystem discovery/path behavior matters.

Bug fixes should normally include regression coverage when practical.

Do not assert private implementation details when observable behavior can be tested instead.

## Verification

Use focused checks while iterating.

Run a specific test with:

```bash
cargo test <test_name>
```

Before declaring Rust work complete, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If platform-specific behavior changed, verify that the implementation remains valid for both Linux and macOS where practical.

If any required verification cannot be run, state exactly what was skipped and why.

## Git and Repository Safety

Preserve unrelated user work.

* Inspect existing changes before broad edits.
* Do not discard unrelated modifications.
* Do not use destructive Git operations merely to obtain a clean tree.
* Do not commit, push, merge, rebase, amend, or force-update history unless explicitly requested.
* Keep generated/build artifacts out of tracked files unless intentionally required.

## Completion

Before declaring an issue complete:

1. Re-read every acceptance criterion.
2. Confirm each criterion is implemented or explicitly report what remains.
3. Run the relevant verification.
4. Check that no unrelated behavior changed.
5. Confirm no new product semantics were silently invented.
6. Report:

   * what changed
   * tests/checks run
   * any remaining limitations, risks, or unresolved blockers

An issue is not complete merely because the code compiles.
