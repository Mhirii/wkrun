# ADR 0001-012: macOS Unsigned Distribution

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

`wkrun` may be distributed to macOS users through GitHub Releases and may potentially use Homebrew later.

The project does not require Mac App Store distribution and does not want Apple Developer Program membership to become a project prerequisite.

Users may need to explicitly approve an unsigned binary through normal macOS security settings.

## Decision

`wkrun` intentionally supports **unsigned and unnotarized macOS binaries**.

- Mac App Store distribution is not required.
- Apple Developer Program membership is not a project requirement.
- Developer ID signing is not required for MVP or normal releases.
- Apple notarization is not required for MVP or normal releases.
- GitHub Release binaries for macOS may be unsigned and unnotarized.

Documentation must explain the expected Gatekeeper flow for users who need to explicitly approve the binary through macOS Privacy & Security settings.

Project documentation must **never** recommend globally disabling Gatekeeper or other system-wide security protections.

Release trust instead relies on the cross-platform controls defined by the release pipeline, including:

- SHA-256 checksums
- build provenance/attestations
- `cargo-auditable`
- protected CI
- verification of the exact published artifacts

Homebrew distribution, if added later, must be evaluated separately against Homebrew's then-current requirements.

Code signing/notarization may become a requirement only through a future explicit project decision.

Tooling defaults must not silently introduce signing/notarization as a release requirement.

## Consequences

### Positive

- macOS releases do not depend on Apple Developer Program membership.
- The project retains an explicit non-App-Store distribution path.
- Users are not instructed to weaken system-wide protections.

### Trade-offs

- macOS users may encounter Gatekeeper warnings and need to approve the binary manually.
- A future distribution channel may require a separate compatibility decision.
