# ADR 0001-004: Staged Quality-Gate Architecture

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Running every available quality tool on every edit would slow development and encourage bypass behavior. Running only cheap checks would provide insufficient assurance for a tool that will execute processes and operate on users' machines.

The project therefore needs increasing levels of assurance as code approaches integration and release.

## Decision

`wkrun` uses a staged quality-gate ladder:

1. **Commit/local fast gate**
   - Cheap checks for rapid developer/agent feedback.

2. **PR comprehensive gate**
   - Full correctness, static analysis, dependency policy, and cross-platform validation.

3. **Merge authoritative gate**
   - Validates the exact integration candidate entering `main`.

4. **Scheduled deep-analysis gate**
   - Expensive bug-finding, safety, and test-strength tools that are not appropriate for every PR.

5. **Tag/release and published-artifact gate**
   - Validates release-mode builds, packaging, provenance, and the exact artifacts users receive.

Required checks fail closed.

Mandatory gates must not use:

- swallowed shell failures
- `continue-on-error`
- retry-until-green behavior for required deterministic tests
- equivalent bypasses that make a failed required check appear successful

CI is authoritative.

Local Git hooks may invoke repository-owned checks, but hooks are optional and are not a hidden source of truth.

The gate ladder must optimize for:

```text
fast iteration early
        +
increasing assurance near release
```

## Consequences

### Positive

- Fast feedback remains fast.
- Expensive analysis can still be aggressive.
- Merge and release evidence are stronger than ordinary local checks.
- Quality tools are less likely to be bypassed because every stage has a clear purpose.

### Trade-offs

- CI configuration has multiple tiers.
- A failure can have different consequences depending on which tier discovers it.
