# wkrun — Product Requirements Document

## 1. Overview

`wkrun` is a local development orchestration tool for projects that require multiple servers, services, or supporting processes to run together.

A typical development environment may require:

* a backend server
* a frontend dev server
* background workers
* databases
* Docker containers
* Docker Compose services
* auxiliary development processes

Without orchestration, these are commonly spread across multiple terminals, tmux panes, or manually managed background processes.

`wkrun` replaces that workflow with a single tool that starts, supervises, observes, and controls the entire local development environment.

The core command is:

```bash
wkrun up
```

The MVP is configuration-driven. Automatic service discovery is post-MVP.

---

# 2. Product Thesis

A development project should be runnable as a single unit.

Instead of:

```text
terminal 1 → backend
terminal 2 → frontend
terminal 3 → docker compose
terminal 4 → worker
terminal 5 → logs
```

the developer should be able to run:

```bash
wkrun up
```

and let `wkrun` own the orchestration.

The domain hierarchy is:

```text
Project
└── Workspace
    └── Service
```

This hierarchy exists from MVP even though advanced multi-workspace functionality, especially Git worktree support, expands after MVP.

---

# 3. Supported Platforms

MVP supports:

* Linux
* macOS

Windows is not an MVP target.

The implementation may rely on Unix-specific primitives such as:

* Unix domain sockets
* Unix process groups
* Unix signals
* Unix filesystem semantics

---

# 4. Goals

The MVP must provide a usable daily-development workflow.

It must:

* run multiple development services together
* support local processes
* support Docker containers
* support Docker Compose services
* support dependency ordering
* support readiness checks
* allocate dynamic host ports
* support fixed host ports
* support environment interpolation
* support service-to-service port interpolation
* supervise services after launch
* automatically restart unexpectedly crashed services
* surface failed, blocked, and degraded states
* persist known projects and workspaces
* expose control through an intuitive CLI
* expose control through a first-class TUI
* allow CLI commands to return while services continue running
* allow the TUI to exit without stopping services
* aggregate and expose logs
* coexist with existing hot-reload tooling
* establish an architecture that naturally supports multiple projects and Git worktrees later

---

# 5. Non-Goals for MVP

The MVP will not:

* implement file watching
* implement source-triggered hot reload
* automatically discover project services
* automatically create Git worktrees
* provide full worktree lifecycle management
* provide advanced HTTP readiness configuration
* provide extensive restart policy customization
* support Windows
* replace Docker Compose
* replace tmux
* replace a terminal emulator
* act as a general terminal multiplexer

Tools such as Vite, Air, Nodemon, and framework development servers continue to own hot reload.

`wkrun` supervises their outer process.

---

# 6. Core Domain Model

## 6.1 Project

A **project** represents a development project known to `wkrun`.

A project contains:

* a stable internal identity
* a canonical filesystem root
* its selected configuration file
* one or more workspaces

Project paths should be canonicalized so accessing the same project through symlinks does not create duplicate registrations.

The directory containing the selected configuration file is the project root. All relative configuration paths resolve from this directory, including process `cwd` values and Compose file paths. Git repository/worktree roots are metadata and do not alter config-relative path resolution.

---

## 6.2 Workspace

A **workspace** represents one independently runnable instance of a project.

For an ordinary non-worktree project, this is the project's normal/default workspace.

With future Git worktree support, each worktree maps to a separate workspace.

A workspace owns or references:

* services
* runtime state
* allocated host ports
* logs
* worktree metadata when applicable
* Docker/Compose resources associated with that workspace

Every registered project and workspace has a stable internal ID persisted in SQLite. Internal IDs must not be derived from mutable values such as branch names. Internal IDs and human-facing workspace names are separate.

---

## 6.3 Service

A **service** is one managed runtime component within a workspace.

A service may be:

* a local process
* a Docker container
* a Docker Compose service

A service may define:

* command/runtime configuration
* dependencies
* named ports
* environment variables
* readiness

---

# 7. Configuration Discovery

Supported configuration filenames, in preferred order:

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

`Workfile` is always TOML.

`Wkrun` is always TOML.

`project.*` is treated specially because the name is generic.

A `project.toml`, `project.yaml`, or `project.yml` file is only treated as a `wkrun` configuration if it positively validates against the `wkrun` schema.

An unrelated `project.*` file must be ignored rather than producing a `wkrun` parsing error.

---

# 8. Config Discovery Traversal

Configuration discovery starts at `$PWD` and walks upward.

When inside a Git repository or worktree, discovery should stop at the current repository/worktree boundary.

The nearest location containing an explicitly wkrun-owned configuration file or a positively validated generic `project.*` configuration wins. At that location, an explicitly wkrun-owned filename is authoritative: if its selected file is malformed or invalid, report that error rather than skipping it and continuing upward. Generic `project.*` files that do not positively validate as wkrun configuration are ignored and discovery continues upward.

If multiple supported configuration files exist in the same directory, the configured priority order is used.

`wkrun` should make the selected configuration file observable in diagnostics/status output.

---

# 9. Configuration Formats

Both TOML and YAML map to the same logical schema.

There are no YAML-specific product semantics.

Example TOML:

```toml
version = 1

[services.db]
type = "compose"
file = "docker-compose.yml"
service = "postgres"

[services.db.ports.postgres]
host = "random"
target = 5432

[services.db.readiness]
tcp = "localhost:${services.db.ports.postgres}"

[services.api]
type = "process"
command = "cargo run"
depends_on = ["db"]

[services.api.ports]
http = "random"

[services.api.env]
PORT = "${services.api.ports.http}"
DATABASE_URL = "postgres://localhost:${services.db.ports.postgres}/app"

[services.api.readiness]
http = "http://localhost:${services.api.ports.http}/health"

[services.web]
type = "process"
command = ["pnpm", "vite", "--port", "${services.web.ports.http}"]
depends_on = ["api"]

[services.web.ports]
http = "random"

[services.web.env]
API_URL = "http://localhost:${services.api.ports.http}"
```

---

# 10. Naming Rules

Service names and port names must be valid interpolation identifiers.

Recommended MVP pattern:

```text
[A-Za-z][A-Za-z0-9_-]*
```

`.` is not allowed because interpolation paths use dots as separators.

Names must be unique within their scope.

---

# 11. Service Types

Every service explicitly declares:

```text
process
docker
compose
```

Runtime type must not be inferred from the presence of fields such as `command`, `image`, or `file`.

Explicit service types improve validation and make schema evolution safer.

---

# 12. Process Services

Example:

```toml
[services.api]
type = "process"
command = "cargo run"
cwd = "./backend"
```

`cwd` is optional.

If omitted, the service runs relative to the project root.

Process services may additionally specify:

* `args`
* `depends_on`
* `ports`
* `env`
* `readiness`

---

# 13. Command Execution Model

Three command forms are supported.

## 13.1 Shell command

```toml
command = "cargo run"
```

String commands are executed using:

```text
/bin/sh -c
```

This provides convenient shell syntax while remaining deterministic across supported platforms.

The user's interactive shell is not implicitly used.

---

## 13.2 Direct argv form

```toml
command = ["cargo", "run"]
```

This bypasses shell parsing.

---

## 13.3 Command plus args

```toml
command = "cargo"
args = ["run", "--release"]
```

When `args` is provided, this is also direct execution and bypasses shell parsing.

Direct execution is preferred where precise quoting and process behavior matter.

---

# 14. Process Groups and Signals

Local services must be launched in identifiable process groups.

This ensures that stopping:

```text
npm → node → vite
```

does not leave child processes running after the parent exits.

Service shutdown should:

1. request graceful termination of the owned process group
2. allow a reasonable grace period
3. force termination if required

Exact grace-period duration may remain implementation-defined for MVP.

---

# 15. Docker Services

Example:

```toml
[services.redis]
type = "docker"
image = "redis:8"
```

Docker services must support enough lifecycle control for:

* start
* stop
* restart
* state inspection
* configured host/container port mappings
* reliable ownership identification

Docker resources created by `wkrun` should be labeled with `wkrun` ownership metadata where practical.

---

# 16. Docker Compose Services

Example:

```toml
[services.db]
type = "compose"
file = "docker-compose.yml"
service = "postgres"
```

A configured Compose service refers to the named service in the user's Compose project.

`wkrun` does not replace Docker Compose.

Compose's own internal dependency graph is respected.

`wkrun` must not use `--no-deps` by default.

Compose-internal dependencies do not automatically become first-class `wkrun` services.

If a Compose dependency is separately declared as a `wkrun` service, wkrun-level lifecycle/readiness semantics apply to that explicit service.

Every wkrun workspace uses a deterministic, Docker-compatible, workspace-specific Compose project name. The exact generated format is an implementation detail. Resources in that namespace are wkrun-managed; manually invoked Compose projects in other namespaces are external and must not be adopted or destroyed.

---

# 17. Compose Port Behavior

Each named wkrun Compose port identifies a TCP container target port:

```toml
[services.db.ports.postgres]
host = "random"
target = 5432
```

If the user's Compose configuration already publishes that target and the mapping is available, it takes priority. After startup, wkrun inspects the resulting container and resolves `${services.db.ports.postgres}` to its actual host-side port. Dynamically allocated Compose host ports may therefore be discovered after startup.

Example:

```yaml
ports:
  - "5432:5432"
```

For MVP, only TCP mappings are supported and each configured target must have exactly one unambiguous host binding. Ambiguous mappings fail clearly rather than being guessed.

If Compose startup fails because the published host port is occupied and `wkrun` configuration provides an alternative/random mapping, generate an ephemeral Compose override file and retry with the configured mapping.

`wkrun` must never modify, rewrite, or `sed` the user's Compose file.

Generated overrides belong to `wkrun` runtime/state storage.

---

# 18. Compose Ownership and Cleanup

`wkrun` only stops or removes Compose resources in its workspace-specific Compose project namespace.

If a wkrun-managed Compose invocation transitively starts dependencies, those resources are wkrun-managed for that workspace.

Resources that already exist in the same workspace-specific Compose project are previously wkrun-managed resources and may be reconciled/adopted. Resources in a manually started or otherwise different Compose project are external and must not be adopted or destroyed.

Concrete container/resource identities should be recorded or otherwise recoverably identifiable. If a Compose startup attempt fails, reconcile resources in the workspace Compose project before retrying; Compose may reuse partially created resources where safe, and those resources remain owned for intentional workspace cleanup.

`wkrun down` must not blindly execute a project-wide:

```bash
docker compose down
```

when doing so could affect resources outside `wkrun` ownership.

---

# 19. Named Ports

Ports are named.

Example:

```toml
[services.api.ports]
http = "random"
debug = 9229
```

Names are used by interpolation.

---

# 20. Fixed Process Ports

For a process service:

```toml
[services.api.ports]
http = 3000
```

means:

> use exactly host port 3000.

If the fixed port is unavailable, startup fails clearly.

`wkrun` must not silently replace a user-specified fixed port with another port.

---

# 21. Random Host Ports

```toml
[services.api.ports]
http = "random"
```

means:

> allocate an available host-machine port.

The global daemon coordinates all `wkrun` allocations, preventing simultaneous workspaces managed by `wkrun` from intentionally receiving the same port.

If a candidate becomes occupied between discovery and actual service binding, `wkrun` may allocate another candidate and retry.

Kernel-level pre-reservation is not required for MVP.

Port search should efficiently skip occupied ports/ranges.

---

# 22. Port Persistence

Random port persistence across `down` is preferred but best-effort for MVP.

Preferred behavior:

1. allocate a random port
2. store the assignment in SQLite
3. on later `up`, attempt to reuse it
4. if occupied, allocate another available port
5. update the stored assignment

This gives a workspace stable URLs when practical.

If preserving random assignments significantly complicates the MVP, reallocating after `down` is acceptable.

---

# 23. Docker Port Mapping

Docker and Compose mappings distinguish host and container-side target ports.

Example:

```toml
[services.db.ports.postgres]
host = "random"
target = 5432
```

If `43128` is allocated:

```text
localhost:43128 → container:5432
```

`host` may be:

* a fixed numeric port
* `"random"`

For mappings created by `wkrun`, `target` is required.

Interpolation resolves to the host-side port:

```text
${services.db.ports.postgres}
```

→

```text
43128
```

---

# 24. Environment Inheritance

Services inherit the environment supplied by the client process that initiates the lifecycle operation.

They must not blindly inherit the potentially stale environment from when the long-running daemon originally started.

When:

```bash
wkrun up
```

is executed:

```text
invoking CLI environment
        ↓
sent to daemon
        ↓
service-specific overrides applied
        ↓
service process
```

Service-specific `env` values override inherited values.

This common environment behavior applies consistently to process, Docker, and Compose services where applicable.

The environment source is always the client process initiating a lifecycle operation. CLI requests send the invoking CLI process environment to the daemon. TUI requests send the environment captured when `wkrun tui` or `wkrun attach` started. The daemon must not substitute its own startup environment, an environment from a previous `up`, or a globally cached environment.

An already-running TUI cannot observe environment changes later made in its parent shell. To apply those values, reopen the TUI from that shell or run the corresponding CLI restart command from that shell.

---

# 25. Interpolation

Interpolation uses:

```text
${...}
```

Supported MVP references include:

```text
${services.api.ports.http}
${env.API_TOKEN}
```

Example:

```toml
[services.web.env]
API_URL = "http://localhost:${services.api.ports.http}"
TOKEN = "${env.API_TOKEN}"
```

`${env.NAME}` resolves from the environment snapshot supplied by the client initiating the lifecycle operation.

Explicitly referencing a missing environment variable is an error.

Interpolation is single-pass and non-recursive for MVP.

Dynamic resource allocation, including random ports, must occur before dependent interpolation values are finalized.

---

# 26. Dependencies

Example:

```toml
[services.api]
depends_on = ["db"]
```

A dependency without readiness unblocks its dependents when it starts successfully.

A dependency with readiness unblocks its dependents only after readiness succeeds.

Dependency cycles are invalid configuration.

---

# 27. Readiness

MVP supports exactly:

* TCP
* HTTP
* command

Only one readiness mechanism may be configured per service.

---

# 28. TCP Readiness

```toml
[services.db.readiness]
tcp = "localhost:${services.db.ports.postgres}"
```

Readiness succeeds when a TCP connection can be established.

---

# 29. HTTP Readiness

```toml
[services.api.readiness]
http = "http://localhost:${services.api.ports.http}/health"
```

MVP success means HTTP status:

```text
200–299
```

Custom accepted status codes, headers, authentication, methods, etc. are post-MVP.

---

# 30. Command Readiness

```toml
[services.worker.readiness]
command = "some-check-command"
```

Readiness succeeds when the command exits successfully.

Command readiness runs from the service's resolved `cwd`, with its resolved environment. String readiness commands use `/bin/sh -c`.

---

# 31. Readiness Timing

Readiness is continuous for the lifetime of a service.

MVP defaults:

```text
poll interval: approximately 500ms
timeout:       30s
```

While the process/container is alive but initial readiness has not succeeded, the service state is:

```text
starting
```

If the 30-second initial readiness timeout expires while the underlying runtime remains alive, the service becomes:

```text
unhealthy
```

`wkrun up` reports this startup outcome as unsuccessful but does not kill or restart the runtime. Probing continues, and one successful probe transitions the service from `unhealthy` to `running`.

After initial readiness succeeds, probes continue. Three consecutive failed probes transition a previously healthy service from `running` to `unhealthy`. One successful probe recovers it to `running`.

Readiness failures do not automatically restart the runtime and do not count toward the eight crash-restart failures.

---

# 32. Service Lifecycle States

MVP includes at least:

```text
starting
running
restarting
stopped
blocked
degraded
unhealthy
failed
```

The persisted desired-state values are `running` and `stopped`. A service's displayed state describes its observed lifecycle and health; a desired-running service may therefore currently be `starting`, `unhealthy`, `blocked`, `degraded`, or `failed`.

---

# 33. Blocked vs Degraded

`blocked` means:

> the service has not started because a required dependency is unavailable, failed, or not ready.

Example:

```text
db      failed
api     blocked
```

`degraded` means:

> the service is already running but a required dependency subsequently became unhealthy.

Example:

```text
db      failed
api     degraded
```

When the dependency recovers:

* blocked services may resume startup
* degraded services return to normal health automatically

Dependency recovery does not restart already-running dependents.

An unhealthy dependency blocks dependents that have not yet started and degrades dependents that are already running. When it returns to `running`, blocked dependents may resume startup and degraded dependents recover automatically.

---

# 34. Unexpected Exit and Restart

Unexpected service exits trigger automatic restart.

Explicit user actions such as:

```text
stop
down
```

must never trigger automatic restart.

A service may automatically restart up to:

```text
8 consecutive failures
```

Example:

```text
api  restarting (5/8)
```

After eight consecutive crash failures:

```text
api  failed
```

Automatic restart stops until explicit user action.

If an unhealthy but alive desired-running runtime later exits unexpectedly, the normal crash restart policy applies.

---

# 35. Restart Counter Reset

MVP reset window:

```text
30 seconds
```

If the service remains successfully running for 30 continuous seconds, its consecutive crash counter resets.

Post-MVP this becomes configurable through mechanisms such as:

```text
WKRUN_RESET_TIME=500ms
WKRUN_RESET_TIME=30s
WKRUN_RESET_TIME=2m
```

and/or user configuration under the platform config directory.

---

# 36. Restart Delay / Backoff

Exact restart delay/backoff is not a major product requirement for MVP.

The implementation should use a simple strategy that prevents an uncontrolled tight restart loop.

The eight-failure cap remains authoritative.

For wkrun-owned Docker and Compose resources, runtime-level automatic restart policies must be disabled so Docker/Compose does not conflict with wkrun's intentional-stop semantics or restart counter. Docker services launch with restart disabled. Compose uses an ephemeral override with `restart: "no"` when necessary; the source Compose file is never modified. External resources retain their existing policies because wkrun does not supervise them.

---

# 37. Hot Reload

`wkrun` does not watch source files or implement hot reload.

Examples of tools expected to continue handling their own hot reload:

* Vite
* Air
* Nodemon
* framework development servers
* compiler watch modes

---

# 38. Startup Atomicity

Workspace startup is not transactional.

If:

```text
A starts
B starts
C fails
```

A and B remain running.

They are not automatically rolled back because C failed.

Services waiting on C remain:

```text
blocked
```

Already-running dependents of C become:

```text
degraded
```

---

# 39. Daemon Architecture

MVP uses **one daemon per operating-system user**.

There is no per-workspace supervisor process model.

Conceptually:

```text
CLI / TUI
    │
    │ Unix domain socket
    ▼
wkrun daemon
    ├── project A
    │   ├── main workspace
    │   └── feat-auth workspace
    └── project B
        └── default workspace
```

The daemon owns live runtime state for all `wkrun` workspaces.

This includes:

* service lifecycle
* process groups
* Docker/Compose lifecycle
* dependencies
* readiness
* restart counters
* port allocation
* live logs/events
* workspace runtime state

---

# 40. Daemon Startup

The daemon starts lazily when first required.

Users should not need to manually start it.

A command such as:

```bash
wkrun up
```

behaves conceptually as:

```text
connect to daemon
      │
      ├─ success → continue
      │
      └─ failure
           ↓
      acquire daemon-start lock
           ↓
      check socket again
           ↓
      spawn daemon if still absent
           ↓
      wait for handshake
           ↓
      release lock
           ↓
      send requested operation
```

The second liveness check after obtaining the lock prevents concurrent CLI calls from spawning duplicate daemons.

---

# 41. Daemon Lifetime

Once started, the daemon remains running even when zero workspaces are active.

A future explicit daemon stop/restart command may be added.

Normal TUI or CLI termination does not terminate the daemon.

---

# 42. IPC

CLI and TUI communicate with the daemon through a Unix domain socket.

Preferred runtime location:

```text
$XDG_RUNTIME_DIR/wkrun/
```

when available.

Fallback:

```text
$TMPDIR/wkrun-$UID/
```

or equivalent private per-user directory.

Socket directories must not be writable by other users.

The IPC protocol may use an internal structured protocol such as JSON messages for MVP.

The exact serialization format is an implementation detail.

---

# 43. Daemon Handshake

A successful handshake is the authority for daemon liveness.

The handshake should include at least:

* protocol version
* daemon instance ID
* daemon PID

A PID or SQLite entry alone never proves the daemon is alive.

---

# 44. Protocol Compatibility

Client and daemon must validate protocol compatibility.

If an incompatible daemon is detected after upgrading `wkrun`, the CLI should use a controlled daemon-restart path.

Restarting/upgrading the daemon must not intentionally stop running services.

---

# 45. Persistent Storage

SQLite is preferred for durable application registry/state.

Persistent data belongs in the platform data directory.

Linux-style path:

```text
$XDG_DATA_HOME/wkrun/wkrun.db
```

with appropriate fallback such as:

```text
~/.local/share/wkrun/wkrun.db
```

macOS should use appropriate local per-user application locations while preserving equivalent semantics.

Possible durable entities include:

```text
projects
workspaces
worktrees
config paths
service metadata
port allocations
historical/last-known metadata
```

Exact SQL schema is an implementation detail.

---

# 46. Runtime / State Storage

Runtime state and logs belong under the platform state location.

Conceptually:

```text
$XDG_STATE_HOME/wkrun/
├── logs/
└── daemon/
```

with Linux fallback:

```text
~/.local/state/wkrun/
```

The project repository should not be polluted with daemon/runtime files.

Local service stdout and stderr must be directed to crash-safe per-service append log files under this state location rather than daemon-owned pipes. The daemon tails those files for CLI and TUI streaming. This allows local services to continue normally if the daemon crashes and allows a replacement daemon to resume log consumption. Preserving stdout/stderr distinction is optional for MVP.

---

# 47. Source of Runtime Truth

SQLite is not authoritative for live runtime state.

The order of authority is:

1. daemon live state
2. actual OS process inspection
3. Docker inspection
4. persisted SQLite metadata as historical/discovery information

Persisted `running` state must not override observable reality.

---

# 48. Daemon Crash Behavior

A daemon crash must not intentionally kill running services.

Local service processes should continue running if the daemon disappears unexpectedly.

Docker/Compose resources naturally continue unless separately terminated.

While the daemon is dead, automatic supervision/restart functionality is temporarily unavailable.

---

# 49. Daemon Recovery / Reconciliation

When a replacement daemon starts:

* confidently identified processes/containers are adopted
* uncertain resources are marked unknown/orphaned and left untouched
* resources proven absent are reconciled as stopped/failed
* stale metadata is corrected

Desired service state (`running` or `stopped`) is durable user intent and must be persisted separately from live state. Intentionally stopped services remain stopped after recovery. Adopted desired-running services remain supervised, and persisted crash/restart metadata should be restored where available so daemon replacement does not trivially reset an existing crash loop.

Never terminate a process merely because a persisted PID matches.

Process IDs may be reused.

Process-group identity and other process metadata should be used where practical.

Docker resources should use ownership labels or concrete resource IDs where practical.

---

# 50. Log Recovery After Daemon Crash

The crash-safe per-service log files allow a replacement daemon to resume reading existing service output. Retention and rotation remain implementation-defined as long as normal development sessions remain useful.

---

# 51. Persistence Across Invocations

`wkrun` remembers known projects and workspaces across invocations.

Persistence may include:

* project identity/path
* workspace identity/path
* worktree relationship
* config path
* historical/random port assignments
* runtime resource metadata
* log locations

Persisted metadata must be reconciled against live reality when used.

---

# 52. CLI Vocabulary

Core MVP commands:

```text
wkrun -h
wkrun help

wkrun up
wkrun up [svc]

wkrun down

wkrun stop <svc>

wkrun restart [svc]
wkrun re [svc]

wkrun logs
wkrun logs [svc]

wkrun ls

wkrun attach
wkrun attach [workspace-or-service]

wkrun tui
```

---

# 53. `wkrun up`

```bash
wkrun up
```

starts services in the current workspace, respecting dependency/readiness ordering.

The command talks to the daemon, waits until every requested service reaches a startup outcome, then exits while services remain running in the daemon.

Successful startup means `running`, including successful readiness where configured. An unsuccessful startup outcome is `failed`, `unhealthy`, or `blocked` because a required dependency is unhealthy or failed. Exit with `0` only when all requested services are `running`; otherwise exit non-zero. Services not directly requested but brought up as dependencies are included in this startup outcome.

`up` is idempotent. Each invocation reloads the current config and sends the initiating client's current environment snapshot to the daemon. Already-running services are not restarted merely because config or environment changed. Stopped or missing services started by the invocation use the latest config and environment; users explicitly restart already-running services to apply changes.

Workspace-wide `up` is an explicit request to bring the whole workspace up. It sets every configured service's desired state to `running`, including services previously stopped individually, and starts them subject to dependency and readiness rules. Daemon recovery alone must not override an intentionally stopped desired state.

---

# 54. `wkrun up <svc>`

```bash
wkrun up api
```

starts a specific service.

Required dependencies are brought up as necessary.

`up <svc>` ensures the service's desired state is running. If its runtime is already alive but `unhealthy`, it does not implicitly restart it; it waits for and reports the existing readiness outcome.

This is also the command used to start a service previously stopped using:

```bash
wkrun stop api
```

No separate `start` command is required for MVP.

---

# 55. `wkrun down`

Stops all wkrun-owned services/resources in the current workspace intentionally.

Services must not automatically restart afterward.

Cleanup must respect ownership boundaries, especially for Compose.

---

# 56. `wkrun stop <svc>`

Stops one service intentionally.

The service remains stopped during normal supervision and daemon recovery until an explicit start operation targets it. `up <svc>`, `restart <svc>`, workspace-wide `up`, and workspace-wide `restart` are explicit start operations.

---

# 57. `wkrun restart`

```bash
wkrun restart
wkrun re
```

restarts all services in the current workspace, respecting dependency ordering.

Workspace-wide restart is an explicit request to restart the whole workspace. It includes every configured service, including services previously stopped individually, and sets their desired state to `running`.

Both workspace-wide and individual restart reload the current selected config from disk and use the environment snapshot from the client initiating the restart. They resolve interpolation, ports, and runtime configuration from that fresh input, intentionally stop the targeted runtime or runtimes, then start them using the newly resolved configuration and environment.

---

# 58. `wkrun restart <svc>`

```bash
wkrun restart api
wkrun re api
```

restarts one service.

An individual restart is an intentional runtime restart followed by the equivalent of `up <svc>`. Required dependencies are brought up when necessary, and readiness evaluation begins again for the restarted service.

---

# 59. `wkrun logs`

```bash
wkrun logs
```

shows combined logs for the current workspace.

Combined logs identify their originating service.

MVP behavior should:

* show a bounded recent history
* then follow new output by default
* allow Ctrl-C to stop following without affecting services

---

# 60. `wkrun logs <svc>`

Shows logs for one service and follows new output by default.

---

# 61. `wkrun ls`

Works globally.

Lists known projects/workspaces and useful runtime status.

It must work even when invoked outside a project.

---

# 62. `wkrun attach`

When invoked inside a recognized project/worktree:

```bash
wkrun attach
```

resolves context from `$PWD` and opens the TUI focused on that workspace.

For a normal non-worktree project, this effectively behaves as:

```bash
wkrun tui
```

---

# 63. `wkrun attach <workspace>`

Opens the TUI directly on the selected workspace.

Workspace arguments resolve by human-facing name within the current project first. Outside a project, a shorthand must be globally unambiguous. Internal workspace IDs may also be accepted as unambiguous targets. Ambiguity is always an error.

`attach` does not modify a global persistent current-workspace setting.

Later CLI commands continue resolving context from `$PWD` or explicit arguments.

---

# 64. `wkrun attach <service>`

When the argument resolves to a service in the current workspace, `attach` opens/focuses that service's logs.

Ambiguous resolution must produce a clear error rather than guessing.

---

# 65. CLI Context Outside a Project

Outside a recognized project/workspace:

```bash
wkrun ls
```

works globally.

```bash
wkrun tui
```

opens the global TUI/project-workspace navigator.

Commands requiring an implicit workspace context should fail clearly:

```text
wkrun up
wkrun down
wkrun logs
wkrun stop api
wkrun restart api
```

Do not silently choose among known workspaces.

Explicit cross-workspace targeting may be added later.

---

# 66. TUI

The TUI is a core interface, not a debugging add-on.

It is:

* keyboard-first
* Vim-first
* not Vim-only

The hierarchy exposed by the TUI is:

```text
Project
└── Workspace
    └── Service
```

---

# 67. TUI Layout

Primary layout:

```text
┌────────────────────────────┬─────────────────────────────┐
│ Services                   │ Selected Service Metadata   │
│                            │                             │
├────────────────────────────┼─────────────────────────────┤
│ Projects / Workspaces      │ Logs                        │
│                            │                             │
└────────────────────────────┴─────────────────────────────┘
```

Primary panes:

* Services
* Metadata
* Projects / Workspaces
* Logs

Visual treatment must clearly indicate:

* selected pane
* focused pane

---

# 68. TUI Pane Navigation

When no pane is focused:

```text
h j k l
```

moves between panes according to spatial direction.

```text
Enter
```

focuses the selected pane.

```text
Esc
```

leaves pane focus or closes the current overlay.

When a pane is focused, `hjkl` operate according to that pane's contents.

---

# 69. Direct Pane Shortcuts

Global shortcuts:

```text
P → Projects / Workspaces
S → Services
M → Metadata
L → Logs
```

These directly focus their target pane.

---

# 70. Services Pane

When focused:

```text
j / k
```

move between services.

```text
l
```

or:

```text
Enter
```

selects the service and focuses/activates its logs.

The selected service also determines Metadata pane content.

---

# 71. Projects / Workspaces Pane

When focused:

```text
j / k
```

move between tree items.

```text
h
```

collapses an expandable item or moves toward its parent.

```text
l
```

expands/descends.

At workspace level, selecting the workspace updates the Services pane.

`Enter` opens/selects the current project/workspace.

---

# 72. Logs Pane

When focused:

```text
j / k
```

scroll vertically.

```text
h / l
```

scroll horizontally.

```text
g
```

goes to the beginning.

```text
G
```

goes to the end/current tail.

---

# 73. Metadata Pane

Displays relevant selected-service information such as:

* service name
* runtime type
* state
* readiness
* uptime
* crash/restart attempts
* PID/container identity
* assigned ports
* dependencies
* command
* working directory

When focused:

```text
j / k
```

scroll vertically.

```text
h / l
```

scroll horizontally when needed.

---

# 74. Search / Filtering

```text
/
```

performs context-sensitive search/filtering.

Examples:

* Logs → search log content
* Services → filter service names
* Projects/Workspaces → search project/workspace names
* Metadata → search visible metadata

```text
n
```

next match.

```text
N
```

previous match.

---

# 75. TUI Lifecycle Actions

Global actions:

```text
r → restart
s → stop
u → start/up
```

Scope must be deterministic.

When Projects/Workspaces pane is focused on a workspace:

```text
r → restart workspace
s → stop workspace
u → start workspace
```

When Services, Metadata, or Logs operate on a selected service:

```text
r → restart selected service
s → stop selected service
u → start selected service
```

TUI lifecycle actions send the TUI process's captured environment snapshot with their request. Service-specific environment configuration overrides that supplied client environment.

Project-wide lifecycle operations are not part of MVP.

---

# 76. Other TUI Keys

```text
Enter → select/open/focus
Esc   → go back / leave focus / close overlay
q     → quit TUI only
?     → help
```

Quitting the TUI does not stop the daemon or workspace services.

Arrow-key/non-Vim alternatives should be supported where practical.

---

# 77. Logging

`wkrun` captures service output for CLI and TUI consumption.

Combined logs identify their source:

```text
api    | INFO listening on :43127
web    | VITE ready in 241ms
db     | database system is ready
```

MVP should preserve service output rather than aggressively rewrite it.

wkrun-added timestamps are optional and may remain implementation-defined.

Historical retention/log rotation may remain implementation-defined as long as normal development sessions remain useful.

---

# 78. Error Handling

Errors must be actionable.

Bad:

```text
invalid config
```

Good:

```text
service "api": depends_on references unknown service "database"
```

Bad:

```text
interpolation failed
```

Good:

```text
service "web": unknown interpolation value:
${services.api.ports.grpc}
```

Errors should identify:

* affected project/workspace
* affected service
* relevant config field
* underlying runtime cause where available

---

# 79. Startup Validation

Before launching services, validate:

* configuration syntax
* configuration version
* service names
* port names
* runtime type
* runtime-specific required fields
* dependency references
* dependency cycles
* readiness definitions
* port definitions
* statically resolvable interpolation references

For MVP, reject unknown fields rather than silently ignoring typos. Environment values must be strings. `args` cannot be combined with array-form `command`; string `command` plus `args` and array-form `command` are direct execution, while a string `command` without `args` uses `/bin/sh -c`.

Dependency cycles are configuration errors.

---

# 80. MVP User Flow

A successful MVP supports this workflow:

```text
enter project
    ↓
wkrun up
    ↓
CLI returns
    ↓
services remain supervised
    ↓
developer continues working
    ↓
wkrun logs / wkrun tui
    ↓
inspect / restart / stop services
    ↓
quit TUI
    ↓
services keep running
    ↓
wkrun down
```

The MVP is not complete merely because it can technically launch multiple child processes.

It must be useful as a daily development tool.

---

# 81. Post-MVP: Automatic Service Detection

`wkrun` should eventually detect likely development services from sources such as:

* `package.json`
* Cargo projects
* Go projects
* Docker Compose
* common framework conventions
* development server configuration

Autodetection must map into the same service model used by explicit configuration.

Explicit configuration remains supported.

---

# 82. Post-MVP: Git Worktree Support

Git worktree support is a major planned capability.

Example:

```text
my-project
├── main
├── feat-auth
└── fix-login
```

Each worktree maps to its own runnable workspace.

---

# 83. Post-MVP: Worktree Port Isolation

Parallel workspaces should automatically avoid host-port conflicts.

Example:

```text
main
  api → 3000
  web → 5173

feat-auth
  api → 3001
  web → 5174

fix-login
  api → 3002
  web → 5175
```

The same interpolation system established in MVP should support these allocations.

---

# 84. Post-MVP: Docker / Compose Worktree Isolation

Parallel workspaces should avoid collisions involving:

* host ports
* Compose project names
* container names
* networks
* workspace-owned Docker resources

Workspace-specific namespacing should be automatic where practical.

---

# 85. Post-MVP: Worktree Workflow

Planned functionality includes:

* discovering worktrees
* listing worktrees/workspaces
* searching workspaces
* attaching to workspaces
* associating worktrees with project workspaces
* automatically starting a workspace after worktree creation
* managing multiple concurrently active workspaces

Exact worktree lifecycle CLI vocabulary is not part of the MVP contract.

---

# 86. Post-MVP: User Configuration

User-global configuration may live under the platform config directory.

Conceptually:

```text
CONFIG_DIR/wkrun/config.toml
```

Potential settings include:

* restart stability window
* readiness timing
* logging preferences
* TUI preferences
* future defaults

Example environment override:

```text
WKRUN_RESET_TIME=45s
```

---

# 87. Architecture Boundaries

Implementation should preserve clear layers:

```text
Config / Discovery
       ↓
Project / Workspace / Service Model
       ↓
Daemon
       ↓
Runtime Adapters
  ├── Process
  ├── Docker
  └── Compose
       ↓
Events / Runtime State
  ├── CLI
  └── TUI
```

CLI and TUI must use the same underlying daemon/runtime model.

The TUI must not directly own child-process lifecycle.

SQLite must not become a substitute for the live runtime model.

---

# 88. MVP Success Criteria

MVP succeeds when a developer with a project containing, for example:

```text
backend
frontend
worker
database
```

can define the stack once and then primarily interact through:

```bash
wkrun up
wkrun logs
wkrun tui
wkrun stop api
wkrun up api
wkrun re api
wkrun down
```

without maintaining multiple terminals solely to keep services alive.

The developer must be able to rely on:

* dependency-aware startup
* readiness checks
* dynamic ports
* fixed ports
* environment interpolation
* local processes
* Docker
* Docker Compose
* automatic crash restart
* clear blocked/degraded/failed states
* persistent project/workspace discovery
* daemon-backed background execution
* combined and per-service logs
* Vim-first TUI interaction
* quitting the TUI without terminating the workspace

---

# 89. License

`wkrun` is licensed under the MIT License.
