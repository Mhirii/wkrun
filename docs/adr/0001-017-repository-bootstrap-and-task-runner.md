# ADR 0001-017: Repository Bootstrap Structure and Task Runner

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Issue #1 should establish the repository infrastructure that is already meaningful while avoiding empty directories and speculative configuration for tools whose activation conditions have not yet been reached.

The project also needs a stable, discoverable human/agent interface for composed repository operations as the number of quality, analysis, and release tasks grows.

## Decision

### Canonical task runner

Adopt `just` from the foundation.

`just` is the canonical human/agent entry point for composed repository operations.

It is a command runner, not a replacement for Cargo and not a second build system.

Canonical operations should be discoverable through:

```bash
just --list
```

The canonical local fast gate is:

```bash
just check
```

as defined in ADR 0001-005.

### What belongs in `justfile`

Simple command composition belongs directly in `justfile`.

Examples include a recipe that runs several existing command-line quality tools in sequence.

If an operation requires substantial:

- shell logic
- branching
- temporary-directory management
- artifact parsing
- complex orchestration

move that implementation into a dedicated script/program and invoke it from `just`.

Do not create one-line wrapper scripts for commands that can be expressed clearly as `just` recipes.

The `justfile` itself must remain formatting-checkable; `just --fmt --check` is an appropriate validation for the task definition.

This ADR does not prescribe a new gate stage for that command beyond requiring that the repository keep the `justfile` valid and format-checkable.

### Required Issue #1 repository infrastructure

Issue #1 should include:

```text
.config/
  nextest.toml

.github/
  workflows/
    ci.yml

src/
  main.rs

.gitignore
_typos.toml
Cargo.lock
Cargo.toml
deny.toml
justfile
LICENSE
rust-toolchain.toml
```

`scripts/` should exist only if Issue #1 actually needs logic too complex for a sensible `just` recipe.

Do not create `scripts/` merely as a placeholder.

### Future source modules

Future architectural modules are introduced when their implementation begins.

Do not create empty placeholders merely to mirror the future architecture.

The intended architecture remains governed by ADR 0001-001.

### CI workflow structure

Keep GitHub Actions understandable.

Initially prefer one CI workflow with separately named jobs over many tiny workflow files.

The exact internal workflow job implementation remains an implementation detail as long as it satisfies the accepted quality-gate ADRs.

### `.gitignore`

Keep `.gitignore` intentional and minimal.

At minimum, ignore Rust build output such as:

```gitignore
/target/
```

Do not ignore `Cargo.lock`; it is committed.

Do not copy a large generic ignore template containing unrelated entries merely for completeness.

### License

Include the actual MIT `LICENSE` file immediately.

### README

A substantive README is not required by this bootstrap decision.

Issue #1 must not invent usage documentation for functionality that is not yet implemented.

## Explicitly Deferred Infrastructure

Deferred infrastructure is intentionally absent from Issue #1.

Its absence is not an oversight.

Where another accepted ADR defines an activation point, that activation remains binding.

**Deferred does not mean optional.**

### Cargo-vet / `supply-chain/`

**Activation:** before Phase 3 — Process Runtime.

**Why deferred:** this is the point where `wkrun` begins executing and supervising user processes and crosses the higher-authority trust boundary identified in the security decisions. Initializing audit/exemption state before the dependency graph has meaningfully stabilized would create premature audit debt.

The binding cargo-vet policy is defined in ADR 0001-008.

### Fuzzing / `fuzz/`

**Activation:** when meaningful parser/decoder surfaces exist, such as configuration parsing, interpolation parsing, or IPC framing/decoding.

**Why deferred:** bootstrap wiring does not yet provide useful fuzz targets. Empty fuzz infrastructure would be ceremonial rather than protective.

### Mutation-testing configuration

**Activation:** once substantive application logic and a sufficiently reliable test suite exist.

**Why deferred:** mutating bootstrap wiring provides little useful semantic signal. Mutation testing becomes valuable for state machines, validation, ownership, reconciliation, allocation, and other meaningful logic.

The scheduled mutation policy is defined in ADR 0001-010.

### AddressSanitizer / ThreadSanitizer configuration

**Activation:** when memory-sensitive and/or concurrency-sensitive implementation exists with meaningful curated targets.

**Why deferred:** sanitizer jobs need real code and curated test surfaces. Empty or ceremonial sanitizer jobs do not improve assurance.

### Miri-specific configuration/scripts

**Activation:** when owned unsafe or suitably low-level code exists that can meaningfully execute under Miri.

**Why deferred:** the bootstrap implementation is expected to remain safe Rust and has no meaningful unsafe safety contract to exercise.

The unsafe/Miri obligations are defined in ADRs 0001-010 and 0001-016.

### `cargo-hack` feature-matrix configuration

**Activation:** when meaningful Cargo feature combinations exist.

**Why deferred:** the project should not create artificial features or an empty matrix merely to exercise tooling.

### Coverage-specific configuration

**Activation:** when enough substantive application code and tests exist for coverage trends to provide useful information.

**Why deferred:** an early bootstrap coverage number would be statistically easy to maximize while saying little about product correctness.

The coverage philosophy is defined in ADR 0001-010.

### `dist` / `cargo-dist` release configuration

**Activation:** when release packaging becomes active.

**Why deferred:** Issue #1 has only a bootstrap executable. Installer/archive configuration should be introduced when there is a meaningful release artifact and release workflow to validate.

The release contract is defined in ADR 0001-011.

### Published-artifact verification workflow

**Activation:** together with the release pipeline.

**Why deferred:** there are no published release artifacts during bootstrap to download and verify.

Post-publication verification remains mandatory once the release pipeline is active, as defined in ADR 0001-011.

## Consequences

### Positive

- The repository contains only infrastructure that is useful now.
- Deferred tooling has explicit activation conditions, so absence cannot be mistaken for abandonment.
- Humans and agents receive one discoverable task interface.
- `scripts/` does not become an accidental shell-based task framework.
- Future security/deep-analysis/release infrastructure is introduced when it can provide meaningful signal.

### Trade-offs

- `just` becomes an additional contributor tool.
- Some infrastructure is intentionally introduced in later issues rather than front-loaded into bootstrap.
