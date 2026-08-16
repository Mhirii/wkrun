# wkrun

Run your entire local development stack with one command.

`wkrun` is a local development orchestrator for projects made up of multiple processes and services: backends, frontend dev servers, workers, databases, Docker containers, Docker Compose services, and more.

Instead of keeping several terminals or tmux panes open:

```text
terminal 1 → cargo run
terminal 2 → pnpm dev
terminal 3 → docker compose up
terminal 4 → worker
```

define your stack once and run:

```bash
wkrun up
```

`wkrun` starts, supervises, and gives you one place to manage everything.

> **Status:** early development. The core design is defined, but the project is not yet stable.

---

## Why wkrun?

Most development projects eventually grow beyond a single command.

You end up managing:

* backend servers
* frontend dev servers
* background workers
* databases
* Docker containers
* Docker Compose services
* logs spread across multiple terminals
* port conflicts
* service startup ordering
* crashed processes

`wkrun` treats them as one runnable development workspace.

```text
Project
└── Workspace
    ├── api
    ├── web
    ├── worker
    └── postgres
```

---

## Features

### Multiple service runtimes

Services can be local processes:

```toml
[services.api]
type = "process"
command = "cargo run"
```

Docker containers:

```toml
[services.redis]
type = "docker"
image = "redis:8"
```

or Docker Compose services:

```toml
[services.db]
type = "compose"
file = "docker-compose.yml"
service = "postgres"
```

---

### Automatic ports

Let `wkrun` find an available host port:

```toml
[services.api.ports]
http = "random"
```

Then reference it elsewhere:

```toml
[services.web.env]
API_URL = "http://localhost:${services.api.ports.http}"
```

Docker ports distinguish host and container ports:

```toml
[services.db.ports.postgres]
host = "random"
target = 5432
```

---

### Dependencies and readiness

Services can depend on each other:

```toml
[services.api]
type = "process"
command = "cargo run"
depends_on = ["db"]
```

Without a readiness check, `api` starts once `db` has started.

With readiness:

```toml
[services.db.readiness]
tcp = "localhost:${services.db.ports.postgres}"
```

`api` waits until `db` is actually ready.

MVP readiness checks:

* TCP
* HTTP
* command

---

### Environment interpolation

Reference host environment variables:

```toml
[services.api.env]
TOKEN = "${env.API_TOKEN}"
```

or runtime values from other services:

```toml
[services.web.env]
API_URL = "http://localhost:${services.api.ports.http}"
```

---

### Process supervision

Unexpected crashes are automatically restarted.

```text
api    restarting (3/8)
```

After 8 consecutive failures:

```text
api    failed
```

A service that stays running for 30 seconds resets its consecutive-failure counter.

An individually stopped service stays stopped during normal supervision and daemon recovery. A workspace-wide `wkrun up` or `wkrun restart` explicitly brings all configured services back up.

`wkrun` does not implement hot reload itself. Tools like Vite, Air, Nodemon, and framework dev servers continue to handle that.

---

### Dependency-aware health

If a dependency fails, its dependents are not killed.

They become degraded instead:

```text
postgres    failed
api         degraded
web         degraded
```

When the dependency recovers, dependents recover automatically without being restarted.

---

## Configuration

`wkrun` supports TOML and YAML.

Recognized filenames include:

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

`Workfile` and `Wkrun` are always parsed as TOML.

Generic `project.*` files are used only when they positively validate as a `wkrun` configuration; unrelated files with those names are ignored.

### Example

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

## CLI

The CLI is intentionally small.

```bash
wkrun up
wkrun up api

wkrun down

wkrun stop api

wkrun restart
wkrun restart api
wkrun re api

wkrun logs
wkrun logs api

wkrun ls
wkrun attach
wkrun attach <workspace>
wkrun attach <service>

wkrun tui

wkrun help
wkrun -h
```

### Examples

Start the current workspace:

```bash
wkrun up
```

Start one service and any required dependencies:

```bash
wkrun up api
```

Follow combined logs:

```bash
wkrun logs
```

View one service:

```bash
wkrun logs api
```

Restart it:

```bash
wkrun re api
```

Stopping or closing the TUI does **not** imply shutting down the workspace.

Use:

```bash
wkrun down
```

when you actually want to stop it.

---

## TUI

`wkrun` includes a keyboard-first, Vim-first terminal interface.

```text
┌────────────────────────────┬─────────────────────────────┐
│ Services                   │ Selected service metadata   │
│                            │                             │
├────────────────────────────┼─────────────────────────────┤
│ Projects / Workspaces      │ Logs                        │
│                            │                             │
└────────────────────────────┴─────────────────────────────┘
```

### Navigation

When navigating between panes:

```text
h j k l     move between panes
Enter       focus selected pane
Esc         leave pane / go back
```

Inside a focused pane, `hjkl` operate on that pane's contents.

Direct pane shortcuts:

```text
P    projects / workspaces
S    services
M    metadata
L    logs
```

Common actions:

```text
r    restart
s    stop
u    start

/    search / filter current context
n    next match
N    previous match

q    quit TUI only
?    help
```

The TUI is Vim-first, not Vim-only; conventional navigation keys should work where practical.

---

## Workspaces

`wkrun` remembers known projects and workspaces across invocations.

A running workspace is supervised independently of the terminal used to launch it, so this works:

```bash
wkrun up
```

and the CLI can return while the workspace continues running.

Later:

```bash
wkrun ls
wkrun attach my-workspace
wkrun tui
```

---

## Roadmap

The initial goal is a solid config-driven multi-service development runner.

Planned post-MVP work includes:

* automatic service detection
* Git worktree integration
* automatic per-worktree port isolation
* Docker / Compose isolation between worktrees
* worktree discovery and management
* starting environments automatically for newly created worktrees
* global user configuration
* configurable restart stability windows
* richer readiness checks

A future setup could look like:

```text
my-project
├── main
│   ├── api :3000
│   └── web :5173
│
├── feat-auth
│   ├── api :3001
│   └── web :5174
│
└── fix-login
    ├── api :3002
    └── web :5175
```

Each worktree gets a complete, conflict-free runnable development environment.

---

## Philosophy

`wkrun` should stay out of the way.

It does not want to replace:

* your shell
* your editor
* tmux
* Docker Compose
* Vite
* Air
* your framework's dev server

It handles the part between them:

> **running and managing the services that make up your local development environment.**

---

## License

`wkrun` is licensed under the [MIT License](LICENSE).
