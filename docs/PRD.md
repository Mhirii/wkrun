# wkrun — Product Requirements Document

## 1. Overview

`wkrun` is a local development orchestration tool for projects that require multiple services or processes to run together.

Typical development environments require developers to manually manage several terminals, tmux panes, Docker Compose sessions, dev servers, workers, databases, and related processes.

`wkrun` replaces that workflow with a single tool that:

* starts and supervises all services required by a project
* allocates conflict-free host ports
* manages local processes, Docker containers, and Docker Compose services
* handles dependencies and readiness
* aggregates and exposes logs
* persists project and workspace information
* provides both a CLI and a terminal user interface
* forms the foundation for isolated, parallel Git worktree environments

The MVP is explicitly configuration-driven.

Automatic project and service detection is planned after MVP.

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

and let `wkrun` manage the runtime environment.

The long-term model is:

```text
Project
└── Workspace
    └── Service
```

A project may eventually contain multiple simultaneously running workspaces, particularly when Git worktrees are used.

---

# 3. Goals

The MVP must provide a genuinely usable local development workflow.

It must:

* run multiple development services together
* support local processes
* support Docker containers
* support Docker Compose services
* supervise services after launch
* automatically restart unexpectedly crashed services
* support dependency ordering
* support service readiness checks
* allocate free host ports automatically
* allow configuration values to reference dynamically allocated ports
* support environment-variable interpolation
* aggregate service logs
* persist known projects and workspaces across invocations
* expose service management through a CLI
* provide a first-class TUI
* allow the TUI to exit without stopping running services
* support navigation between projects, workspaces, services, metadata, and logs
* remain compatible with existing development hot-reload tools

---

# 4. Non-Goals for MVP

The MVP will not:

* implement source-file watching
* implement its own hot-reload system
* automatically infer project services
* automatically create Git worktrees
* automatically isolate Docker resources across worktrees
* automatically resolve worktree-specific port conflicts beyond normal workspace port allocation
* restart dependent services when a dependency restarts
* provide advanced HTTP readiness configuration
* provide extensive user-configurable restart policies
* require a global always-running daemon
* replace Docker Compose
* replace tmux or a terminal emulator
* provide a full terminal multiplexer

Tools such as Vite, Air, Nodemon, framework-specific dev servers, and similar utilities continue to own hot reload.

`wkrun` supervises the outer process.

---

# 5. Core Domain Model

## 5.1 Project

A **project** represents a development project known to `wkrun`.

A project has:

* an identity
* a filesystem root
* a discovered configuration file
* one or more workspaces

For MVP, most projects will normally have one workspace.

---

## 5.2 Workspace

A **workspace** represents one runnable instance of a project.

A workspace owns runtime state including:

* services
* allocated host ports
* running processes
* container instances
* logs
* supervisor state
* workspace metadata

The abstraction must exist in MVP even though advanced multi-workspace functionality is primarily intended for post-MVP Git worktree support.

---

## 5.3 Service

A **service** is one managed runtime component inside a workspace.

A service may be:

* a local process
* a Docker container
* a Docker Compose service

A service may define:

* dependencies
* ports
* environment variables
* readiness
* runtime-specific options

---

# 6. Configuration Discovery

`wkrun` must automatically search for supported project configuration files.

Supported filenames are:

```text
wkrun.toml
wkrun.yaml
wkrun.yml

workrun.toml
workrun.yaml
workrun.yml

project.toml
project.yaml
project.yml

Workfile

Wkrun
```

`Workfile` must contain TOML.

`Wkrun` must contain TOML.

The extensionless forms are therefore parsed exclusively as TOML.

If multiple supported files exist in the same project, `wkrun` must use a deterministic priority order.

Recommended priority:

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

If multiple files are present, `wkrun` should surface which file was selected.

---

# 7. Configuration Format

Both TOML and YAML represent the same underlying schema.

The configuration must contain a version and named services.

Example:

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
command = "pnpm vite --port ${services.web.ports.http}"
depends_on = ["api"]

[services.web.ports]
http = "random"

[services.web.env]
API_URL = "http://localhost:${services.api.ports.http}"
```

---

# 8. Service Types

Every service must explicitly declare its runtime type.

Supported MVP values:

```text
process
docker
compose
```

Runtime type must not be inferred from the presence of fields such as `command` or `image`.

Explicit types improve validation, error reporting, and future schema extension.

---

# 9. Process Services

A local process service uses:

```toml
[services.api]
type = "process"
command = "cargo run"
cwd = "./backend"
```

Required:

```text
type
command
```

Optional:

```text
cwd
depends_on
ports
env
readiness
```

If `cwd` is omitted, the process runs relative to the project/workspace root.

---

# 10. Docker Services

A Docker service runs a standalone Docker container.

Example:

```toml
[services.redis]
type = "docker"
image = "redis:8"
```

Required:

```text
type
image
```

Docker runtime options may be expanded later, but the MVP must support enough functionality to start, stop, restart, inspect, and expose configured ports for a container.

---

# 11. Docker Compose Services

A Compose service references a service inside an existing Compose file.

Example:

```toml
[services.db]
type = "compose"
file = "docker-compose.yml"
service = "postgres"
```

Required:

```text
type
file
service
```

`wkrun` does not replace Compose.

It orchestrates selected Compose services as members of the `wkrun` workspace.

---

# 12. Ports

## 12.1 Named Ports

Ports are named.

Example:

```toml
[services.api.ports]
http = "random"
debug = 9229
```

Named ports make runtime interpolation explicit and readable.

---

## 12.2 Random Ports

The value:

```text
"random"
```

always means:

> Allocate an available port exposed on the user's host machine.

Example:

```toml
[services.api.ports]
http = "random"
```

may resolve to:

```text
43127
```

The resolved value is then available through interpolation.

---

## 12.3 Docker Port Mapping

Docker and Compose services distinguish the host port from the container target port.

Example:

```toml
[services.db.ports.postgres]
host = "random"
target = 5432
```

If `wkrun` allocates `43128`, the mapping is:

```text
localhost:43128 → container:5432
```

The interpolated service port refers to the host-side port:

```text
${services.db.ports.postgres}
```

resolves to:

```text
43128
```

---

# 13. Interpolation

Configuration values may reference runtime and environment values.

Interpolation syntax:

```text
${...}
```

---

## 13.1 Service Port References

Example:

```text
${services.api.ports.http}
```

---

## 13.2 Host Environment References

Example:

```text
${env.API_TOKEN}
```

---

## 13.3 Example

```toml
[services.web.env]
API_URL = "http://localhost:${services.api.ports.http}"
API_TOKEN = "${env.API_TOKEN}"
```

Interpolation must occur only after all required dynamic resources, such as random ports, have been resolved.

Interpolation errors must produce clear validation or startup errors.

---

# 14. Dependencies

Services may declare dependencies:

```toml
[services.api]
depends_on = ["db"]
```

Dependency semantics are:

### Dependency without readiness

The dependent may start once the dependency has successfully started.

### Dependency with readiness

The dependent may start only once the dependency has passed its readiness check.

Example:

```text
db starts
↓
db readiness succeeds
↓
api starts
```

Dependencies also affect runtime health state.

---

# 15. Readiness

A service may define one readiness mechanism.

Supported MVP readiness types:

```text
TCP
HTTP
command
```

Only one readiness mechanism may be configured per service.

---

## 15.1 TCP Readiness

```toml
[services.db.readiness]
tcp = "localhost:${services.db.ports.postgres}"
```

Readiness succeeds once a TCP connection can successfully be established.

---

## 15.2 HTTP Readiness

```toml
[services.api.readiness]
http = "http://localhost:${services.api.ports.http}/health"
```

For MVP, readiness succeeds when the endpoint returns a successful HTTP status.

Advanced options such as custom accepted status codes, headers, request methods, retry configuration, and authentication are post-MVP.

---

## 15.3 Command Readiness

```toml
[services.worker.readiness]
command = "some-check-command"
```

Readiness succeeds when the command exits successfully.

---

# 16. Service Lifecycle

## 16.1 Unexpected Exit

Unexpected service exits automatically trigger restart attempts.

This applies to services that are intended to be running.

---

## 16.2 Intentional Stop

If the user explicitly stops a service, `wkrun` must not restart it automatically.

The supervisor must therefore distinguish between:

```text
unexpected exit
```

and:

```text
intentional stop
```

---

## 16.3 Restart Limit

A service may automatically restart up to:

```text
8 consecutive failures
```

Example:

```text
api    restarting (5/8)
```

After eight consecutive failures:

```text
api    failed
```

Automatic restarting stops and the failure is surfaced to the user.

---

## 16.4 Failure Counter Reset

For MVP, a service is considered stable after:

```text
30 seconds
```

of successful continuous runtime.

After this period, the consecutive-failure counter resets.

Post-MVP, this duration becomes configurable through:

```text
WKRUN_RESET_TIME
```

using duration values such as:

```text
500ms
30s
2m
```

and/or through:

```text
CONFIG_DIR/wkrun/config.toml
```

The exact post-MVP configuration precedence may be determined later.

---

# 17. Service States

The MVP must expose at least:

```text
starting
running
restarting
stopped
failed
degraded
```

Readiness may additionally be represented separately or as part of the starting state.

---

## 17.1 Starting

The service is being launched or is waiting to satisfy readiness.

---

## 17.2 Running

The service is alive and its dependency requirements are healthy.

---

## 17.3 Restarting

The service exited unexpectedly and is undergoing an automatic restart attempt.

---

## 17.4 Stopped

The service is intentionally not running.

---

## 17.5 Failed

The service itself could not remain running after eight consecutive failures.

---

## 17.6 Degraded

The service itself may still be running, but one or more required dependencies are unhealthy or failed.

Example:

```text
db      failed
api     degraded
web     degraded
```

---

# 18. Dependency Failure Propagation

Dependency failures must not automatically stop dependents.

Dependency failures must not automatically restart dependents.

Instead:

```text
dependency fails
↓
dependent remains alive
↓
dependent becomes degraded
```

When the dependency recovers:

```text
dependency recovers
↓
dependent automatically returns to normal health
```

No dependent restart occurs unless explicitly requested by the user.

---

# 19. Hot Reload

`wkrun` must not implement source watching or automatic source-triggered process restarts.

Existing development servers already provide this functionality.

Examples include:

```text
Vite
Air
Nodemon
framework dev servers
watch-mode compilers
```

`wkrun` manages the service process itself and must avoid conflicting with the service's own reload system.

---

# 20. Runtime Supervision

The MVP does not require a single global always-running daemon.

However, `wkrun up` must be useful as a CLI command and must not require a permanently open TUI.

Therefore, an active workspace requires a lightweight supervisor that remains alive while its services are running.

Conceptually:

```text
wkrun up
   │
   ├── workspace supervisor
   │      ├── api
   │      ├── web
   │      ├── worker
   │      └── db
   │
   └── CLI invocation may return
```

The workspace supervisor owns:

* process lifecycle
* automatic restarts
* service state
* readiness
* dependency state
* log capture
* runtime communication
* allocated runtime resources

The TUI and CLI communicate with this active workspace state.

---

# 21. TUI Exit Behavior

Quitting the TUI must not stop running services.

The key:

```text
q
```

means:

> Quit the TUI only.

It does not mean:

```text
wkrun down
```

Service lifecycle must be explicitly controlled through lifecycle commands or actions.

---

# 22. Persistence

`wkrun` must remember projects and workspaces across invocations.

Persistent state may include:

* known projects
* project paths
* known workspaces
* workspace paths
* selected config file
* allocated ports
* supervisor identity
* supervisor communication endpoint
* log locations
* relevant runtime metadata

Persistence should use appropriate platform state/config directories rather than polluting the project directory unless explicitly required.

Conceptually:

```text
$XDG_STATE_HOME/wkrun/
├── projects/
├── workspaces/
├── supervisors/
└── logs/
```

The exact on-disk representation is an implementation detail.

Persisted state must tolerate stale supervisor or process information.

`wkrun` must verify runtime reality rather than blindly trusting persisted process IDs.

---

# 23. CLI

The CLI must prioritize intuitive naming and predictable behavior.

Core MVP vocabulary:

```text
wkrun -h
wkrun help

wkrun up
wkrun up [svc]

wkrun down

wkrun stop [svc]

wkrun restart [svc]
wkrun re [svc]

wkrun logs
wkrun logs [svc]

wkrun ls

wkrun attach [workspace]

wkrun tui
```

---

# 24. CLI Semantics

## `wkrun -h`

Display concise help.

---

## `wkrun help`

Display CLI help.

Subcommand-specific help should follow standard CLI expectations where supported.

---

## `wkrun up`

Start the current workspace.

Services must be started in dependency-respecting order.

The command must not require the TUI to remain open.

---

## `wkrun up [svc]`

Start a specific service.

Required dependencies must also be brought up as necessary.

---

## `wkrun down`

Stop all services belonging to the current workspace.

The shutdown is intentional, so services must not be automatically restarted.

---

## `wkrun stop [svc]`

Stop one service intentionally.

The service remains stopped until explicitly started again.

---

## `wkrun restart [svc]`

Restart the selected service.

Alias:

```text
wkrun re [svc]
```

---

## `wkrun logs`

Display combined workspace logs.

Logs should identify their originating service.

---

## `wkrun logs [svc]`

Display logs for one service.

---

## `wkrun ls`

List known projects and workspaces along with meaningful runtime status.

Exact presentation may evolve, but it should make active environments easy to discover.

---

## `wkrun attach [workspace]`

Attach or switch context to an existing workspace.

This must work with persisted and active workspace information.

---

## `wkrun tui`

Open the interactive terminal UI.

When possible, it should enter the current project/workspace context automatically.

---

# 25. TUI Goals

The TUI is a core product interface, not an optional debugging frontend.

It should allow the developer to navigate among:

```text
projects
workspaces
services
service metadata
logs
```

without switching terminals.

The design must be:

```text
keyboard-first
Vim-first
not Vim-only
```

Arrow keys and intuitive alternatives should be supported where practical.

---

# 26. TUI Layout

The primary layout is:

```text
┌────────────────────────────┬─────────────────────────────┐
│ Services                   │ Selected Service Metadata   │
│                            │                             │
├────────────────────────────┼─────────────────────────────┤
│ Projects / Workspaces      │ Logs                        │
│                            │                             │
└────────────────────────────┴─────────────────────────────┘
```

The four primary panes are:

```text
Services
Metadata
Projects / Workspaces
Logs
```

Pane borders or equivalent visual treatment must clearly indicate:

* currently selected pane
* currently focused pane

---

# 27. TUI Navigation Model

The TUI has two navigation levels:

## Pane Selection

When no pane is focused:

```text
h j k l
```

move selection between panes according to their spatial relationship.

Example:

```text
h → pane to the left
j → pane below
k → pane above
l → pane to the right
```

---

## Pane Focus

Pressing:

```text
Enter
```

focuses the selected pane.

Once focused, `hjkl` operate within that pane according to its content.

Pressing:

```text
Esc
```

returns to pane-level navigation or closes the current overlay/context.

---

# 28. Direct Pane Shortcuts

The following global shortcuts always focus the associated pane:

```text
P → Projects / Workspaces
S → Services
M → Metadata
L → Logs
```

These shortcuts provide fast navigation without requiring repeated pane movement.

---

# 29. Services Pane

The Services pane displays services belonging to the active workspace.

Example:

```text
● api       running      :43127
● web       running      :43128
● postgres  running      :43129
○ worker    stopped
```

When focused:

```text
j / k
```

move through services.

```text
l
```

or:

```text
Enter
```

selects the service and focuses or activates its corresponding logs.

The selected service also controls what is displayed in the Metadata pane.

---

# 30. Projects / Workspaces Pane

The pane presents hierarchical project/workspace information.

Example:

```text
my-app
├── main
├── feat-auth
└── fix-login

other-project
└── main
```

When focused:

```text
j / k
```

move through items.

```text
h
```

collapses an expandable item or moves toward its parent.

```text
l
```

expands an expandable item or descends into it.

On the lowest relevant workspace level, selecting or entering the workspace updates the Services pane to represent that workspace.

`Enter` selects or opens the current project/workspace context.

---

# 31. Logs Pane

The Logs pane displays either:

* combined workspace logs
* logs for the currently selected service

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

moves to the beginning.

```text
G
```

moves to the end/current tail position as appropriate.

---

# 32. Metadata Pane

The Metadata pane displays information about the selected service.

Possible information includes:

* service name
* runtime type
* state
* readiness state
* uptime
* restart attempts
* PID or container identifier
* assigned ports
* dependency state
* command
* working directory
* relevant runtime metadata

When focused:

```text
j / k
```

scroll vertically.

```text
h / l
```

may scroll horizontally when required.

---

# 33. Search and Filtering

The key:

```text
/
```

performs context-sensitive search or filtering based on the currently focused pane.

Examples:

### Logs

Search log contents.

### Services

Filter/search services.

### Projects / Workspaces

Search project and workspace names.

### Metadata

Search visible metadata fields.

After search:

```text
n
```

moves to the next match.

```text
N
```

moves to the previous match.

---

# 34. TUI Lifecycle Actions

Global lifecycle actions operate on the currently meaningful selected scope.

```text
r → restart
s → stop
u → start/up
```

---

## Service Selected

```text
r → restart service
s → stop service
u → start service
```

---

## Workspace Selected

```text
r → restart all workspace services
s → stop all workspace services
u → start all workspace services
```

Workspace-level actions must respect dependencies.

Project-wide lifecycle operations are not required for MVP.

---

# 35. TUI Other Keys

```text
Enter → select/open/focus
Esc   → go back / leave focus / close overlay
q     → quit TUI only
?     → show help
```

Destructive or broad operations may request confirmation where appropriate.

---

# 36. Logging Requirements

`wkrun` must capture service output.

It should preserve the distinction between services and ideally stdout/stderr.

Combined logs must clearly identify their originating service.

Example:

```text
api    | INFO listening on :43127
web    | VITE ready in 241ms
db     | database system is ready
api    | GET /api/me 200
```

Logs must remain available to both:

```text
wkrun logs
```

and the TUI.

Persistence duration and log rotation policy may remain implementation-level decisions for MVP, provided normal development sessions remain usable.

---

# 37. Error Handling

Configuration and runtime errors must be actionable.

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

Failures should identify:

* affected project/workspace
* affected service
* relevant field
* underlying runtime cause where available

---

# 38. Startup Validation

Before launching services, `wkrun` should validate:

* configuration syntax
* configuration version
* service names
* service types
* dependency references
* dependency cycles
* readiness definitions
* port definitions
* interpolation references that can be statically validated
* required runtime-specific fields

Dependency cycles must be rejected.

---

# 39. Shutdown Behavior

`wkrun down` must shut down workspace services intentionally and cleanly.

Where possible:

1. request graceful service termination
2. allow a reasonable grace period
3. force termination if necessary
4. mark services stopped
5. prevent automatic restart

Process-tree cleanup must avoid leaving orphan development processes behind.

---

# 40. Architecture Requirements

The implementation should keep these conceptual layers separate:

```text
Config
  ↓
Workspace / Service Model
  ↓
Supervisor
  ↓
Runtime Adapters
  ├── Process
  ├── Docker
  └── Compose
  ↓
Events / State
  ├── CLI
  └── TUI
```

The TUI must not directly own process lifecycle logic.

The CLI and TUI should both act through the same underlying state and supervisor abstractions.

This makes future daemon extraction possible without redesigning the entire application.

---

# 41. MVP Usability Standard

The MVP is not considered complete merely because it can technically launch multiple commands.

A usable MVP must allow a developer to:

```text
enter a project
↓
run wkrun up
↓
return to their shell
↓
inspect services
↓
inspect logs
↓
open and close the TUI
↓
stop/restart individual services
↓
stop the workspace
↓
later invoke wkrun again and rediscover the project/workspace
```

without manually managing multiple terminal sessions.

---

# 42. Post-MVP: Automatic Detection

Post-MVP, `wkrun` should be capable of detecting likely services from project files.

Potential sources include:

* package manager scripts
* Cargo projects
* Go projects
* Docker Compose files
* common framework conventions
* development server configuration

Automatic detection should produce or internally map into the same service model used by explicit configuration.

The config-driven system remains the canonical foundation.

---

# 43. Post-MVP: Git Worktree Support

Git worktree support is a major planned capability.

The intended model is:

```text
Project
├── main workspace
├── feat-auth workspace
└── fix-login workspace
```

Each worktree should be capable of running a complete independent development environment.

---

# 44. Post-MVP: Worktree Port Isolation

Parallel worktrees frequently attempt to bind the same host ports.

`wkrun` should resolve these conflicts automatically.

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

Dynamic values should continue to resolve through the same interpolation system established in MVP.

---

# 45. Post-MVP: Docker Isolation Across Worktrees

Parallel worktrees must not collide on:

* container names
* Compose project names
* networks
* exposed host ports
* other workspace-scoped Docker resources

`wkrun` should automatically create workspace-specific Docker/Compose namespaces.

Example:

```text
myapp-main-postgres-1
myapp-feat-auth-postgres-1
myapp-fix-login-postgres-1
```

The user should not need to manually construct these namespaces.

---

# 46. Post-MVP: Worktree Management

Planned worktree-related capabilities include:

* discovering worktrees
* listing worktrees
* associating worktrees with workspaces
* creating runnable workspace contexts
* searching workspaces
* attaching to workspaces
* starting services when a worktree is created
* managing multiple simultaneously active workspaces

The exact CLI vocabulary for worktree creation/removal may be designed when implementation begins.

---

# 47. Post-MVP: Global Configuration

User-wide configuration should be supported under the platform configuration directory.

Conceptual location:

```text
CONFIG_DIR/wkrun/config.toml
```

Potential settings include:

* restart stability period
* default runtime behavior
* logging preferences
* UI preferences
* future global defaults

Environment variables may override selected configuration values where appropriate.

Example:

```text
WKRUN_RESET_TIME=45s
```

---

# 48. Post-MVP: Global Daemon

MVP uses on-demand workspace supervision.

A later version may introduce a global daemon:

```text
wkrund
├── project registry
├── workspace registry
├── supervisors
├── runtime state
└── logs

      ▲
      │ IPC
      │
wkrun CLI / TUI
```

Potential benefits:

* centralized project discovery
* centralized workspace supervision
* easier multi-project navigation
* cleaner attach behavior
* unified logs
* reduced duplicated supervisor infrastructure

The MVP architecture must not require a global daemon, but should avoid choices that make one difficult to introduce later.

---

# 49. Success Criteria

The MVP succeeds if a developer with a project containing a backend, frontend, worker, and database can define those services once and then use:

```bash
wkrun up
```

instead of manually maintaining multiple terminal sessions.

They must be able to:

* leave the launching terminal workflow
* reopen `wkrun`
* see the project and workspace
* inspect all running services
* view combined or per-service logs
* stop/start/restart services
* use the Vim-first TUI
* rely on automatic restart after crashes
* rely on dependency readiness
* use dynamic ports without manually finding unused ones
* use Docker and Compose services alongside local processes
* quit the TUI without terminating their development environment

The MVP should feel like a tool that can remain part of a developer's daily workflow, not a proof of concept.

---

# 50. Product Direction

The MVP establishes this abstraction:

```text
one workspace
=
one independently managed runnable instance of a project
```

The initial version solves multi-service local development.

Future versions extend the same abstraction to:

```text
multiple projects
×
multiple worktrees
×
multiple isolated runtime environments
```

without requiring developers to manually manage terminal panes, conflicting ports, conflicting Docker resources, or runtime state.
