# wkrun — Roadmap to MVP

## Goal

Ship an MVP that is usable as a daily local-development runner, not merely a proof of concept.

The MVP is complete when a developer can:

```bash
wkrun up
wkrun logs
wkrun tui
wkrun stop api
wkrun up api
wkrun restart api
wkrun down
```

and reliably manage a multi-service project containing local processes, Docker services, and Docker Compose services.

---

# Phase 0 — Project Foundation

Build the basic project structure and shared domain model.

### Implement

* Rust workspace/crate structure
* error/reporting conventions
* logging/tracing for `wkrun` itself
* core types:

  * Project
  * Workspace
  * Service
  * ServiceState
  * DesiredState
  * RuntimeType
  * PortSpec
  * ReadinessSpec
* stable internal project/workspace IDs
* platform path helpers for:

  * config
  * data
  * state
  * runtime/socket paths
* Linux/macOS platform abstraction

### Exit criteria

* project builds cleanly
* core domain types are defined without CLI/TUI coupling
* platform directories resolve correctly on Linux and macOS
* service state machine is represented explicitly

---

# Phase 1 — Config Discovery, Parsing, and Validation

Make `wkrun` understand projects before it can run anything.

### Implement config discovery

Supported files:

```text
wkrun.toml
wkrun.yaml
wkrun.yml

workrun.toml
workrun.yaml
workrun.yml

Workfile
Wkrun

project.toml
project.yaml
project.yml
```

Rules:

* discovery starts at `$PWD`
* walk upward to Git worktree/repository boundary
* nearest applicable config wins
* malformed explicit wkrun files produce errors
* invalid generic `project.*` files are ignored
* config directory becomes project root

### Implement schema

Support:

* `version`
* `services`
* runtime type:

  * process
  * docker
  * compose
* `command`
* `args`
* `cwd`
* `depends_on`
* `ports`
* `env`
* `readiness`

### Implement validation

Reject:

* unknown fields
* invalid service/port identifiers
* missing runtime-specific fields
* dependency cycles
* unknown dependencies
* invalid port declarations
* conflicting command forms
* multiple readiness mechanisms
* invalid interpolation references where statically detectable

### Implement interpolation

Support:

```text
${env.FOO}
${services.api.ports.http}
```

Rules:

* single-pass
* non-recursive
* missing referenced env variables are errors

### Exit criteria

A real project config can be discovered, parsed, normalized into the internal model, and validated without starting services.

---

# Phase 2 — Persistence and Daemon Skeleton

Establish the architecture before implementing runtime behavior.

### Implement SQLite registry

Persist at least:

* projects
* workspaces
* config paths
* workspace paths
* desired service state
* port allocations
* daemon/runtime metadata needed for reconciliation

Do not use SQLite as live runtime truth.

### Implement daemon

One daemon per OS user.

Behavior:

```text
CLI
 ↓
connect to daemon
 ↓
if unavailable:
  acquire startup lock
  check again
  spawn daemon
  handshake
 ↓
request
```

### Implement IPC

Support:

* daemon handshake
* protocol version
* daemon instance ID
* basic request/response framing
* request errors
* graceful client disconnect

Initial operations may simply be:

```text
ping
status
shutdown-daemon/internal
```

### Implement daemon lifetime

* daemon starts lazily
* daemon remains alive with zero active workspaces
* CLI exit does not affect daemon
* incompatible protocol can trigger controlled daemon replacement

### Exit criteria

```bash
wkrun <basic-command>
```

can transparently start/connect to the daemon, query it, disconnect, and reconnect later.

No services need to run yet.

---

# Phase 3 — Process Runtime

Get the core `wkrun up` experience working for normal local processes.

### Implement command execution

Support:

```toml
command = "cargo run"
```

via:

```text
/bin/sh -c
```

Support direct execution:

```toml
command = ["cargo", "run"]
```

and:

```toml
command = "cargo"
args = ["run"]
```

### Implement environment resolution

Lifecycle request supplies client environment snapshot.

Resolve:

```text
client env
+
service env overrides
+
runtime interpolation
```

### Implement process groups

* each local service gets an identifiable process group
* graceful stop targets entire group
* forced termination fallback
* no orphan Vite/Node/Air children

### Implement crash-safe logging

Prefer:

```text
service stdout/stderr
→ state-dir log file
```

Daemon tails files rather than relying exclusively on fragile daemon-owned pipes.

### Implement basic lifecycle

* start
* stop
* restart
* detect unexpected exit
* desired state
* actual state

### Implement crash restart

* unexpected exit → automatic restart
* max 8 consecutive failures
* 30s stable runtime resets counter
* simple backoff to avoid tight loop
* explicit stop disables auto-restart

### Exit criteria

A config containing several local process services can be started and supervised after the launching CLI exits.

---

# Phase 4 — Dependencies, Ports, and Readiness

Turn process launching into actual orchestration.

## Dependency graph

Implement:

* dependency-aware startup ordering
* blocked state
* degraded propagation
* recovery

Rules:

```text
not started + bad dependency → blocked
already running + bad dependency → degraded
```

Do not kill or restart dependents automatically.

## Port allocator

Implement global daemon-managed allocation.

Support:

```toml
http = 3000
```

and:

```toml
http = "random"
```

Rules:

* fixed port means exactly that port
* occupied fixed port fails clearly
* random port allocates available host port
* avoid conflicts between wkrun workspaces
* retry another candidate if race occurs
* best-effort reuse persisted allocation

## Readiness

Implement:

* TCP
* HTTP
* command

Defaults:

```text
poll interval ≈ 500ms
initial timeout = 30s
HTTP success = 200–299
```

Behavior:

```text
starting
→ running
```

or:

```text
starting
→ unhealthy
```

Continuous health:

* keep probing after initial success
* 3 consecutive failures → unhealthy
* one success → recover
* running dependents → degraded
* blocked dependents may start after recovery

Readiness failure does not trigger crash restart.

### Exit criteria

A process-only project can define realistic dependencies and dynamic ports and behave predictably under startup failure, crashes, and readiness loss.

At this point, `wkrun` should already be useful for many projects.

---

# Phase 5 — CLI MVP

Build the complete intended CLI around the daemon/runtime model.

### Implement

```bash
wkrun -h
wkrun help

wkrun up
wkrun up <svc>

wkrun down

wkrun stop <svc>

wkrun restart
wkrun restart <svc>

wkrun re
wkrun re <svc>

wkrun logs
wkrun logs <svc>

wkrun ls

wkrun attach
wkrun attach <workspace-or-service>

wkrun tui
```

### Required semantics

#### `up`

* reload config
* send caller environment
* start requested scope
* wait until requested services reach startup outcome
* return 0 only when requested services are running
* non-zero on failed/blocked startup

#### `restart`

Equivalent to:

```text
intentional stop
+
fresh up
```

Therefore reload:

* config
* caller environment
* interpolation

Workspace-wide `up` and `restart` explicitly start all configured services, including previously intentionally stopped ones.

#### `logs`

* bounded recent history
* follow by default
* Ctrl-C only disconnects log client

#### `ls`

* global
* works outside projects

#### Outside project

Only globally meaningful commands work without project context.

### Exit criteria

The process-runtime path is fully controllable without using the TUI.

---

# Phase 6 — Docker Runtime

Add standalone Docker services.

### Implement

* image launch
* env propagation
* fixed/random host port mappings
* container target ports
* lifecycle:

  * start
  * stop
  * restart
* ownership labels
* state inspection
* crash/restart semantics
* intentional-stop behavior
* readiness against Docker services
* restart policy disabled for wkrun-owned containers

### Reconciliation

Daemon restart should be able to:

* identify owned containers
* adopt them
* avoid unrelated containers
* reconcile missing containers

### Exit criteria

Docker services behave like process services from the user's perspective.

The same:

```text
up
stop
restart
readiness
dependencies
logs/status
```

model works consistently.

---

# Phase 7 — Docker Compose Runtime

Implement Compose carefully because ownership matters.

### Compose workspace namespace

Every workspace uses a deterministic wkrun-specific Compose project name.

Do not share the namespace used by ordinary manual Compose invocations.

### Service startup

For:

```toml
type = "compose"
file = "compose.yml"
service = "postgres"
```

run the named Compose service.

Respect Compose's own dependencies.

Do not use `--no-deps` by default.

### Existing Compose ports

If Compose already exposes the configured target port and it works:

* use it
* inspect actual host binding
* expose that binding through wkrun interpolation

If there is an occupied host-port conflict and wkrun config defines a fallback:

* generate ephemeral Compose override
* retry with fixed/random wkrun host port

Never modify the user's Compose file.

### Restart-policy override

For wkrun-managed Compose resources, override automatic Docker restart behavior where needed so wkrun remains lifecycle authority.

### Ownership

Resources under the workspace-specific Compose project are wkrun-managed.

Resources in unrelated/manual Compose projects are external.

Failed startup resources remain identifiable under the workspace namespace and can be reconciled before retry.

### Cleanup

Do not blindly use project-wide `docker compose down` if ownership could extend beyond intended resources.

Only clean wkrun-owned workspace resources.

### Exit criteria

A mixed project containing:

```text
process
Docker
Compose
```

services works under the same dependency/readiness/lifecycle model.

---

# Phase 8 — TUI MVP

Build the primary interactive interface after the daemon and lifecycle APIs are stable.

## Layout

```text
┌────────────────────────────┬─────────────────────────────┐
│ Services                   │ Selected Service Metadata   │
│                            │                             │
├────────────────────────────┼─────────────────────────────┤
│ Projects / Workspaces      │ Logs                        │
│                            │                             │
└────────────────────────────┴─────────────────────────────┘
```

## Pane navigation

Unfocused:

```text
h j k l     move between panes
Enter       focus
Esc         back/unfocus
```

Direct focus:

```text
P    projects/workspaces
S    services
M    metadata
L    logs
```

## Services

```text
j/k        move
l/Enter    select and focus logs
```

## Projects/workspaces

```text
j/k        move
h          collapse / parent
l          expand / descend
Enter      select
```

## Logs

```text
j/k        vertical scroll
h/l        horizontal scroll
g/G        top/bottom
```

## Search

```text
/          contextual search/filter
n/N        next/previous
```

## Lifecycle

```text
r          restart
s          stop
u          up/start
```

Scope:

* workspace selected in Projects pane → workspace
* selected service context → service

## Other

```text
q          quit TUI only
?          help
```

TUI lifecycle requests send the environment snapshot captured when the TUI process started.

### Exit criteria

A developer can spend a normal development session in the TUI without needing separate terminals solely for service lifecycle/log management.

---

# Phase 9 — Recovery, Edge Cases, and Hardening

Before calling the project MVP, deliberately break it.

### Daemon recovery

Test:

* daemon killed while services run
* daemon upgraded/restarted
* services survive
* replacement daemon adopts known resources
* uncertain processes are not killed
* desired states survive
* restart counters survive where practical

### Concurrent operations

Test:

* two simultaneous `wkrun up`
* simultaneous port allocation
* repeated `up`
* `restart` during startup
* `down` while readiness is pending
* multiple CLI/TUI clients

### Config changes

Test:

* change config while service is running
* `up` leaves existing running service untouched
* `restart` applies new config/env
* malformed config errors clearly

### Runtime failures

Test:

* fixed port occupied
* random port race
* dependency crash
* dependency recovery
* readiness timeout
* readiness loss/recovery
* 8 crash loop
* nested child process cleanup
* Docker restart policy conflict
* partial Compose startup

### Persistence

Test:

* stale DB state
* moved/deleted project
* stale workspace
* stale PID
* stale Docker resources
* symlinked project paths

### Exit criteria

No common failure mode requires manually killing forgotten processes, deleting internal state, or repairing Docker resources.

---

# Phase 10 — MVP Polish and Release

### Documentation

Ship:

* README
* config reference
* CLI reference
* TUI key reference
* example configurations:

  * process-only
  * Docker
  * Compose
  * mixed stack
* troubleshooting guide

### UX polish

Ensure errors answer:

```text
what failed?
which service?
why?
what can I do?
```

Examples:

```text
service "api": fixed port 3000 is already in use
```

rather than:

```text
bind error
```

### Packaging

At minimum:

* reproducible release binaries for Linux/macOS
* installation instructions
* MIT LICENSE
* version command
* shell-completion support only if inexpensive; not required for MVP

### Final MVP acceptance test

A fresh user should be able to:

1. install `wkrun`
2. create `wkrun.toml`
3. run `wkrun up`
4. return to their shell while services stay running
5. use dynamic ports and interpolation
6. observe dependency/readiness behavior
7. view logs
8. open and quit the TUI
9. stop/restart individual services
10. use process, Docker, and Compose services together
11. kill/restart the daemon without automatically killing the application stack
12. run `wkrun down` and leave no wkrun-owned runtime garbage behind

If those all work reliably, the MVP is done.
