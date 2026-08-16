# ADR 0001-013: Post-Merge and Deep-Analysis Failure Policy

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

Scheduled/deep analysis runs after ordinary PR/merge validation and may discover risk after `main` is otherwise green.

Treating every such finding as equally catastrophic would create noise and potentially paralyze development.

Treating them as merely informational would make the deep-analysis system meaningless.

## Decision

Deep-analysis findings are classified as:

- **Critical**
- **Serious**
- **Signal**

### Critical

Critical findings immediately block releases.

Examples include:

- demonstrated security issues
- undefined behavior
- memory corruption
- reproducible data races
- deterministic correctness failures
- release/published-artifact verification failures

If a Critical issue affects an already published release, treat it as a release incident and determine whether a corrective release is required.

### Serious

Serious findings require investigation and a tracking issue.

They normally block release until:

- resolved, or
- explicitly reclassified with justification

Examples may include:

- meaningful surviving mutations
- deterministic fuzz crashes
- important feature-combination failures
- persistent sanitizer failures whose impact is not yet classified
- substantial unexplained quality regression in critical code

### Signal

Signal-level findings remain visible technical-quality debt but do not automatically block development or release.

Examples may include:

- modest coverage regressions
- unmaintained dependency warnings
- low-value/questionable mutation findings
- non-correctness quality trends

### Development health vs release readiness

A deep-analysis failure does not automatically:

- revert `main`
- prevent unrelated development
- mean every ordinary development workflow must stop

Development health and release readiness are separate concepts.

`main` may remain usable for development while being explicitly **release-blocked**.

### Issue handling

Repeated CI runs must not create duplicate tracking issues for the same unresolved failure.

High-confidence reproducible failures may create a tracking issue immediately.

Transient/infrastructure failures should first be confirmed.

### Evidence retention

Every deep-analysis failure must retain enough evidence for reproduction and investigation, including where applicable:

- commit SHA
- platform/toolchain
- tool version
- command/configuration
- logs
- failing test information
- failure artifacts

Specific requirements:

- fuzz crashes retain their reproducer/corpus input
- mutation failures retain the exact surviving mutation
- sanitizer/Miri failures retain diagnostic output

Fixes should add permanent regression coverage where practical.

### Release policy

Agents may not dismiss a scheduled-check failure merely because ordinary PR/merge CI remains green.

A release may not proceed while unresolved:

- Critical blockers
- applicable Serious blockers

remain.

## Consequences

### Positive

- Deep analysis has real enforcement power.
- Noisy signals do not automatically stop unrelated development.
- Release readiness reflects risks discovered after merge.
- Reproducibility evidence is retained.

### Trade-offs

- Findings require classification and sometimes explicit reclassification.
- Release state is more nuanced than a single green/red `main` badge.
