# ADR 0001-010: Scheduled Deep-Analysis Gate

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Ordinary PR tests and static analysis cannot efficiently expose every class of defect relevant to `wkrun`.

The project wants deeper analysis for:

- weak tests
- undefined behavior
- memory errors
- data races
- unexpected input space
- coverage blind spots
- feature-combination breakage

These tools can be too expensive or specialized for every PR.

## Decision

Scheduled validation is split into nightly and weekly work.

### Nightly, where applicable

#### Coverage

Use `cargo-llvm-cov` for coverage and trend visibility.

Coverage is initially informational.

No arbitrary global percentage threshold is imposed.

#### AddressSanitizer

Run AddressSanitizer against a curated sanitizer-compatible test suite.

#### ThreadSanitizer

Run ThreadSanitizer independently against concurrency-relevant tests.

#### Miri

Run focused Miri tests for owned unsafe/low-level code.

Any owned `unsafe` code must have:

- an explicit safety rationale
- appropriate focused validation

Miri should be used wherever that safety contract can meaningfully be exercised.

The entire integration suite is not required to run under Miri.

#### Fuzzing

Once suitable fuzz targets exist, run bounded scheduled fuzzing.

Likely future target classes include:

- config parsing
- interpolation parsing
- IPC decoding/protocol framing
- similar parser/decoder surfaces

Failing corpus/reproducer inputs must be retained.

#### Feature combinations

Once meaningful Cargo feature combinations exist, validate them using `cargo-hack`.

If there is no meaningful feature matrix, no ceremonial `cargo-hack` job is required.

### Weekly

Run `cargo-mutants` over meaningful application logic.

Particularly valuable future target areas include:

- state machine behavior
- desired-state logic
- dependency propagation
- config validation
- ownership decisions
- port allocation
- reconciliation
- restart counters

Surviving meaningful mutants require:

- stronger tests, or
- a narrow documented exclusion

Mutation score alone is not considered sufficient evidence.

### General rules

Once a deep-analysis tool/target has been declared supported, its failures are real engineering failures rather than informational decoration.

Tool-specific exclusions must be narrow and justified.

Failures discovered by:

- sanitizers
- Miri
- fuzzing
- mutation testing

should result in permanent regression coverage where practical.

Expensive tooling must target code where it provides meaningful signal rather than being run ceremonially.

MemorySanitizer is not part of the initial required suite. It may be introduced later when reliable full instrumentation is practical.

## Intended assurance model

```text
ordinary CI      -> expected behavior
mutation testing -> test strength
Miri / ASan      -> memory and safety violations
TSan             -> data races
fuzzing          -> unexpected input space
coverage         -> blind spots
```

## Consequences

### Positive

- Deep failures become visible without slowing every PR.
- Test quality is measured semantically through mutation testing.
- Unsafe/concurrent code receives specialized validation.

### Trade-offs

- Nightly/weekly infrastructure is more complex.
- Some tools require curated test targets and/or nightly Rust.
