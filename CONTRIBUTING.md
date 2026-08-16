# Contributing to wkrun

Thanks for helping build `wkrun`, a Linux/macOS local-development orchestrator.

## Start Here

Work from an issue when one exists. Before writing code:

1. Read the issue's complete acceptance criteria.
2. Read the relevant parts of [`docs/PRD.md`](docs/PRD.md).
3. Inspect the affected code and tests.
4. Stop and raise an explicit question if the issue and PRD conflict, or if required product behavior is unspecified.

The PRD defines user-visible behavior. [`ROADMAP.md`](ROADMAP.md) defines implementation sequencing; it does not override the PRD. Keep changes limited to the issue and its necessary prerequisites.

Read [`AGENTS.md`](AGENTS.md) for the complete project guardrails. Do not change the PRD, roadmap, or agent guide unless the task explicitly calls for it.

## Architecture Decisions

Architecture Decision Records live in [`docs/adr/`](docs/adr/). Name each ADR using:

```text
[issue number]-[ADR number relative to that issue]-[name].md
```

Use zero-padded numeric fields. The first ADR for issue `#24`, for example, is:

```text
docs/adr/0024-01-name.md
```

If your change introduces or revises an architectural decision, record it in the appropriate ADR instead of documenting it only in code or a PR discussion.

## Development

`wkrun` is a Rust 2024 binary crate. Useful commands:

```bash
cargo build
cargo run
cargo test <test_name>
```

Before submitting Rust changes, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If you cannot run a relevant check, say which one and why in the PR.

## Do Not Break These

- The core model is `Project -> Workspace -> Service`.
- One per-user daemon owns live runtime state; CLI and TUI are clients, not lifecycle owners.
- SQLite persists registry, intent, allocations, and history. It is not live runtime truth.
- Desired state and observed runtime/health state are distinct.
- A daemon crash must not intentionally terminate running services.
- Establish ownership before stopping or deleting processes, containers, or Compose resources. A stored PID alone is not ownership proof.
- Lifecycle actions use the initiating client's environment snapshot, not the daemon's startup environment.
- Relative paths resolve from the selected configuration file's directory, not from the Git root.
- Explicit `wkrun.*` configuration errors; generic `project.*` files are ignored unless they positively validate as wkrun configuration.
- `wkrun` supervises outer development processes. Hot reload and file watching remain the responsibility of the managed tool.

The details live in the PRD; do not replace them with an easier implementation.

## Tests That Matter

Tests should prove acceptance criteria and observable behavior. For runtime work, cover the relevant failure and recovery paths—not only successful startup.

Pay particular attention to:

- desired versus observed state transitions;
- intentional stops versus crashes;
- daemon recovery and stale persisted state;
- resource ownership and cleanup;
- repeatable lifecycle actions and concurrent daemon/port operations;
- dependency, readiness, and partial-start recovery;
- process-tree cleanup and Docker/Compose isolation.

Prefer deterministic coordination to arbitrary sleeps. Use real temporary directory trees for config discovery or path-resolution tests. Include a regression test for a bug fix when practical.

## Review Before a PR

Review the diff for scope creep, accidental product decisions, and missed tests. Do not include unrelated refactors or generated build artifacts.

If you use this repository's OpenCode setup:

- `review` is the normal pre-PR reviewer.
- `audit` performs adversarial, evidence-based inspection.
- `/verify-audit` independently validates audit findings; audit findings are hypotheses until verified.
- `final-review` is for milestone-ending or architecturally significant changes.

## Pull Request Checklist

- [ ] The change satisfies the issue acceptance criteria.
- [ ] It preserves the relevant PRD behavior.
- [ ] Tests cover the important success, failure, and recovery behavior.
- [ ] Formatting, Clippy, and tests pass—or skipped checks are explained.
- [ ] The PR contains no unrelated changes, generated artifacts, or unacknowledged risks.
