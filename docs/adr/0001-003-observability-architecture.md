# ADR 0001-003: Observability Architecture

- **Status:** Accepted
- **Issue:** #1 — Bootstrap Rust project and quality gates
- **Date:** 2026-08-16

## Context

`wkrun` will be a long-running orchestration tool responsible for process lifecycle, dependency decisions, readiness, IPC, Docker/Compose operations, persistence, and recovery.

Observability is therefore an architectural requirement rather than incidental debugging output.

The project also has a strong performance requirement: instrumentation must provide deep diagnostic value without becoming a correctness dependency or causing significant hot-path work.

## Decision

### Canonical instrumentation stack

Application code uses `tracing` as the canonical instrumentation API.

The baseline stack includes:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
tracing-appender = "0.2"
```

OpenTelemetry trace export is a first-class capability from the foundation through:

- `tracing-opentelemetry`
- OpenTelemetry SDK
- OTLP export

Application code must instrument through `tracing`; it must not directly create OpenTelemetry spans.

OpenTelemetry/OTLP is an optional subscriber/export layer. Normal `wkrun` operation must not require an OTLP collector.

Trace export must use asynchronous/batched processing rather than synchronous export on application paths.

Failure, unavailability, or saturation of telemetry export must never fail or materially block normal lifecycle operations.

### Internal telemetry and managed-service output are separate

```text
wkrun internal behavior
    -> tracing spans/events

managed service stdout/stderr
    -> service log files / log streaming
```

Managed service output must not be converted into ordinary `wkrun` tracing events merely for forwarding.

### Logging quality requirement

Logging must explain useful:

- state
- decisions
- outcomes
- failures

Logs should not exist merely to announce function entry.

With `DEBUG` enabled, a developer should be able to follow important control flow and understand what `wkrun` is doing and why without attaching a debugger.

Important non-obvious decisions should normally be observable at `DEBUG` or a higher appropriate level, including:

- dependency decisions
- reasons a service was blocked or not started
- port selection/rejection
- retries and retry reasons
- state transitions
- resource adoption or refusal to adopt
- configuration selection
- reconciliation decisions
- recovery behavior
- meaningful operation outcomes

### Level semantics

#### `ERROR`

An operation cannot complete as intended, or an unexpected internal failure/invariant violation occurred.

#### `WARN`

Something abnormal happened, but `wkrun` can continue, recover, retry, or safely degrade.

#### `INFO`

Low-frequency, meaningful lifecycle and control-plane events.

Normal `INFO` output must remain readable.

#### `DEBUG`

Detailed diagnostic information sufficient to reconstruct important control flow, decisions, sanitized inputs, outcomes, and useful timings.

Useful diagnostic information must not all be hidden behind `TRACE`.

#### `TRACE`

Fine-grained or high-frequency execution detail for deep debugging and performance analysis, such as:

- individual readiness probes
- IPC frames/messages
- polling iterations
- inspection details
- candidate allocation attempts

High-frequency activity must not emit at `INFO`.

### Structured fields

Prefer structured fields over embedding values into message strings.

Canonical field names should be reused consistently when the concept applies:

```text
request_id
project_id
workspace_id
service
runtime
daemon_instance_id
pid
container_id
port_name
host_port
attempt
state
desired_state
elapsed_ms
```

State transitions should include both the previous and new state where applicable.

Meaningful operations should expose durations through spans and/or structured events.

### Spans and events

Spans represent meaningful units of work.

Events represent occurrences within that work.

Expected future span concepts include:

```text
cli.command
daemon.request
workspace.up
service.start
service.stop
service.restart
runtime.spawn
readiness.probe
port.allocate
config.load
config.validate
```

### Instrumentation performance rules

Observability must not accidentally become significant application work.

- Do not apply bare `#[instrument]` indiscriminately.
- Prefer `skip_all` with explicitly selected structured fields.
- Do not implicitly format large config objects, environment maps, request structures, or similar values merely because a function is instrumented.
- Avoid expensive diagnostic-value construction when the corresponding level is disabled.
- Diagnostic output should not stall lifecycle supervision under sustained logging pressure.
- If diagnostic telemetry is dropped under saturation, that loss must itself be observable/accounted for.
- `DEBUG` and `TRACE` instrumentation remains compiled into release builds and is controlled through runtime filtering.

### Secret handling

Never log or trace:

- environment variable values
- credentials
- tokens
- authorization headers
- other known secret-bearing values

Environment variable names/counts may be recorded when useful, but values must not be.

Complete command lines, URLs, config objects, or arbitrary user structures must not be recorded unless their contents are known to be safe.

### Competing frameworks

Do not introduce another competing logging/tracing framework without an explicit architectural decision.

## Consequences

### Positive

- Debug logging is operationally useful rather than decorative.
- Traces can be exported to standard OpenTelemetry tooling for visualization.
- Instrumentation remains decoupled from any one trace backend.
- Performance-sensitive paths retain control over instrumentation cost.
- Service logs and `wkrun` diagnostics retain distinct semantics.

### Trade-offs

- Instrumentation requires discipline around field selection, levels, and secret handling.
- Export infrastructure must be treated as best-effort observability rather than part of correctness.
