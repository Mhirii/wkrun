# ADR 0001-011: Release and Published-Artifact Gate

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

A successful source-tree build does not prove that the archive users download is correct.

The release artifact itself is the product.

The release pipeline therefore needs to validate:

- release identity
- exact built binaries
- dependency metadata
- checksums
- provenance
- package extraction
- platform behavior
- the exact published bytes

## Decision

### Release identity

Stable releases use immutable tags of the form:

```text
vX.Y.Z
```

The following versions must agree exactly:

```text
Git tag
=
Cargo package version
=
wkrun --version
```

Release tags must point to commits that passed the protected integration path.

Releases must never be built from uncommitted/local source state.

Published tags/releases are not silently rewritten to replace bad artifacts. Corrections receive a new version.

### Release orchestration

Use `cargo-dist` / `dist` as the release orchestration layer.

Supported Linux/macOS release targets must be explicitly configured.

The release tool orchestrates project policy; generated/default behavior does not override project decisions.

### Auditable binaries

Release binaries are built with `cargo-auditable` dependency metadata.

Candidate binaries are audited from the binary itself before publication.

### Checksums

Every release archive receives a SHA-256 checksum.

The release provides a unified checksum manifest.

Download/install flows must verify checksums where applicable.

### Build provenance

Release artifacts receive build-provenance attestations using GitHub Artifact Attestations.

### Candidate artifact smoke tests

Each packaged artifact is:

1. extracted into a clean temporary environment
2. executed from the extracted package
3. smoke-tested on its target platform

The smoke test must use the packaged executable, not a convenient `target/release/wkrun` binary.

At minimum, initial smoke tests verify:

- the executable exists and can run
- executable permissions are correct where applicable
- `wkrun --help` succeeds
- `wkrun --version` succeeds
- the reported version exactly matches the release version

Smoke testing expands as stable product functionality becomes available.

### Cross-platform validation

Linux and macOS artifacts are independently validated.

A successful Linux artifact does not prove that the macOS artifact is acceptable, or vice versa.

### Publication order

Publication occurs only after all required candidate artifacts pass validation.

### Post-publication verification

After publication:

1. download the artifacts from the actual public release path
2. verify SHA-256 checksums
3. verify provenance attestations
4. extract the downloaded package
5. rerun smoke tests against the downloaded executable

A release is not considered fully validated until the published artifact has passed this verification.

A post-publication verification failure is a release failure/incident and must not be silently ignored.

## Consequences

### Positive

- The exact bytes users receive are verified.
- Dependency metadata is embedded in binaries.
- Provenance and integrity are independently checkable.
- CI cannot accidentally test one artifact and publish another without detection.

### Trade-offs

- Releases have more steps than a simple `cargo build --release`.
- Publication verification requires additional CI/runtime capacity.

## Related decision

macOS signing/notarization policy is defined separately in ADR 0001-012.
