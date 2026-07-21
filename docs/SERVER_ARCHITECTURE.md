# Server Architecture

See also:

- [`SERVER_SERVICE_SPLIT_PLAN.md`](./SERVER_SERVICE_SPLIT_PLAN.md)
- [`SWARM_ARCHITECTURE.md`](./SWARM_ARCHITECTURE.md)

## Overview

jcode uses a **single-server, multi-client** architecture. One `jcode serve`
process manages all sessions and state. Automation clients (debug CLI, swarm
workers, local tooling) connect over a Unix socket and can reconnect after
disconnects.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SERVER (🔥 blazing)                              │
│                                                                             │
│  jcode serve                                                                │
│  ├── Unix socket:  /run/user/$UID/jcode.sock                                │
│  ├── Debug socket: /run/user/$UID/jcode-debug.sock                          │
│  ├── Registry:     ~/.jcode/servers.json                                    │
│  ├── Provider (Claude/OpenAI/OpenRouter)                                    │
│  ├── MCP pool (shared across sessions)                                      │
│  └── Sessions:                                                              │
│        ├── 🦊 fox   (active)  → "🔥 blazing 🦊 fox"                         │
│        ├── 🐻 bear  (active)  → "🔥 blazing 🐻 bear"                        │
│        └── 🦉 owl   (idle)    → "🔥 blazing 🦉 owl"                         │
└─────────────────────────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
    ┌─────────┐   ┌─────────┐   ┌─────────┐
    │ Client 1│   │ Client 2│   │ Client 3│
    │ 🦊 fox  │   │ 🐻 bear │   │ 🦉 owl  │
    └─────────┘   └─────────┘   └─────────┘
```

## Naming

```
SERVER = Adjective/Verb modifier          SESSIONS = Animal nouns
────────────────────────────              ────────────────────────
🔥 blazing   ❄️ frozen   ⚡ swift          🦊 fox    🐻 bear   🦉 owl
🌀 rising    🍂 falling  🌊 rushing        🌙 moon   ⭐ star   🔥 fire
✨ bright    🌑 dark     💫 spinning       🐺 wolf   🦁 lion   🐋 whale

Combined: "🔥 blazing 🦊 fox" = server + session
```

The server gets a random adjective/verb name on startup (e.g., "blazing").
Each session gets an animal noun (e.g., "fox"). Together they form a natural
phrase in logs and debug output: "🔥 blazing 🦊 fox".

The server records its name in the registry (`~/.jcode/servers.json`). Stale
entries are cleaned up automatically.

## Lifecycle

The daemon lifecycle belongs to the process supervisor (Docker, systemd, CI).
`jcode serve` runs in the foreground of its container or service unit. There
are no `server start|stop` management commands and no in-place hot reload:
upgrades happen by replacing the binary or image and restarting the process.

### Server Startup

Start the daemon explicitly:

```bash
jcode serve
```

Long-lived deployments can give the daemon a stable client-visible identity with
`jcode serve --server-name <name>` or the `JCODE_SERVER_NAME` environment
variable. The optional `JCODE_SERVER_DISPLAY_NAME` environment variable is also
accepted for service managers that prefer a display-oriented name. CLI input wins
over environment input. Names are normalized to registry-safe lowercase labels,
so `mount-cloud/fabian` displays as `mount-cloud-fabian`.

### Server Shutdown

The server shuts down when:
- **Idle timeout**: temporary servers exit after their idle window
- **Signal**: SIGTERM triggers a graceful shutdown (used by `docker stop`)

### Remote Client Working Directory

By default, a client sends its current working directory to the server when it
subscribes, and the server uses that as the session working directory. Socket
forwarding wrappers for remote daemons can keep the client and server paths
separate with `--remote-working-dir`:

```bash
jcode --socket /tmp/jcode.sock -C /local/checkout --remote-working-dir /remote/checkout
```

`-C` must exist on the client. `--remote-working-dir` must be an absolute path
that exists on the server.

### Client Reconnection

Automation clients retry with exponential backoff when the connection drops.
On reconnect they resume the same session, because session state persists on
disk.

## Socket Paths

```
/run/user/$UID/
├── jcode.sock          # Main communication socket
└── jcode-debug.sock    # Debug/testing socket
```

## Key Behaviors

| Scenario | Behavior |
|----------|----------|
| `jcode serve` | Runs the daemon in the foreground |
| Kill a client | Server + other clients unaffected |
| SIGTERM to server | Graceful shutdown |
| Resume session | `jcode --resume fox` reconnects to existing session |
