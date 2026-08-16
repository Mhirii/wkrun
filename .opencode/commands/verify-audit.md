---
description: Independently verify the latest adversarial audit findings and review for missed issues
agent: review
---

Independently verify the most recent `audit` agent report from this conversation.

Treat every audit finding as an UNTRUSTED HYPOTHESIS. Do not accept a claim merely because the audit
agent reported it.

Read `AGENTS.md`, the relevant `docs/PRD.md` sections, the affected source code, tests, and current
changes before reaching conclusions.

For every audit finding:

1. Locate the exact cited code and inspect the surrounding implementation.
2. Verify the claimed execution path, state transition, ownership relationship, or concurrency
   interleaving.
3. Verify any external assumptions using authoritative sources when necessary.
4. Attempt reproduction or run focused tests/commands when practical.
5. Compare the claimed behavior against the PRD when product semantics are involved.
6. Classify the finding as:
   - CONFIRMED
   - PARTIALLY CONFIRMED
   - REJECTED
   - UNVERIFIABLE
7. Explain the classification with concrete evidence.
8. Correct the severity if the audit overstated or understated it.
9. If confirmed, state the smallest correct fix direction and the regression test that should prove it.

Do not modify files.

After verifying every audit finding, perform your OWN independent review of the affected code.

The audit agent is intentionally adversarial and may:
- produce false positives,
- miss bugs,
- misunderstand ownership,
- overstate severity,
- make incorrect assumptions about Rust, Unix, Docker, Compose, SQLite, or concurrency.

Therefore do not limit your review to its findings.

Specifically look for issues the audit missed involving:

- correctness
- security
- races and concurrency
- daemon lifecycle and recovery
- desired vs observed state
- process ownership and cleanup
- stale persistence
- Docker/Compose ownership
- resource leaks
- stability
- performance with plausible real impact
- Linux/macOS portability
- PRD violations
- missing regression/acceptance tests

## Output

Start with:

# Audit Verification Verdict

Give one overall verdict:

- READY
- READY AFTER CONFIRMED FIXES
- NOT READY

Then:

## Audit Finding Verification

For every original finding, preserve its audit ID and report:

### AUDIT-XXX — <title>

**Verdict:** CONFIRMED | PARTIALLY CONFIRMED | REJECTED | UNVERIFIABLE  
**Corrected severity:** Critical | High | Medium | Low | None

**Verification**

Concrete evidence supporting your verdict.

**Required action**

What must happen before merge, or `None` if rejected.

**Regression test**

The test needed if confirmed, or `None`.

Then:

## Additional Findings

Report independently discovered issues using the normal review severity format.

Do not repeat already-confirmed audit findings here.

Then:

## Final Merge Blockers

List only issues that actually block the PR.

If none:

`None.`

Be skeptical of the audit and skeptical of the code. The objective is accuracy, not agreement with
either one.
