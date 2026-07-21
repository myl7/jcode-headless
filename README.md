# jcode

jcode is a headless coding-agent runtime. It is designed for Docker, CI,
background workers, and local Unix-socket automation. This tree does not ship
a TUI, desktop app, browser login, remote web UI, or interactive REPL.

## Runtime surfaces

- `jcode run --json "..."` returns one machine-readable result.
- `jcode run --ndjson "..."` streams machine-readable events.
- `jcode serve` runs the long-lived local daemon on its Unix socket. The
  daemon lifecycle belongs to the container supervisor (for example Docker),
  so there are no start/stop management commands.
- `jcode auth status|doctor` validates configured credentials.
- `jcode auth import claude|openai` imports an already-authenticated
  subscription credential. It never starts a login flow.

Running `jcode` without a command prints help and exits with an error.

## Credentials

Authentication must be completed outside the headless runtime. The preferred
container boundary is one writable `JCODE_HOME` volume. jcode stores native
subscription credentials in:

- `$JCODE_HOME/auth.json` for Claude subscriptions.
- `$JCODE_HOME/openai-auth.json` for ChatGPT/Codex subscriptions.

The volume must be writable. OAuth providers rotate refresh tokens, and jcode
persists the replacement token to its native account file.

To bootstrap from official CLI credentials, pipe the credential JSON over
stdin into the explicit import command, then start the runtime:

```bash
# Claude Code credential
docker run --rm -i \
  -v jcode-state:/var/lib/jcode \
  jcode:headless auth import claude --stdin \
  < "$HOME/.claude/.credentials.json"

# Codex credential
docker run --rm -i \
  -v jcode-state:/var/lib/jcode \
  jcode:headless auth import openai --stdin \
  < "$HOME/.codex/auth.json"
```

The import copies only credential data into the writable jcode store and does
not print tokens. A read-only file mount under `$JCODE_HOME/external` is also
supported when stdin is not convenient.

API providers remain supported. Supply their normal environment variables,
for example `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`, or
`GEMINI_API_KEY`, or use the non-interactive provider configuration commands.

See [OAUTH.md](OAUTH.md) for credential layout and rotation details.

## Docker

Build the minimal image:

```bash
docker build -t jcode:headless .
```

The default image omits the optional local embedding, PDF, and Bedrock feature
stacks. Enable any of them at build time when required:

```bash
docker build \
  --build-arg JCODE_FEATURES=pdf,embeddings,bedrock \
  -t jcode:headless-full .
```

Run one request without publishing a port:

```bash
docker run --rm \
  -v jcode-state:/var/lib/jcode \
  -v "$PWD:/workspace" \
  -w /workspace \
  jcode:headless run --json "Inspect this repository and report its status"
```

Run the long-lived daemon:

```bash
docker run -d --name jcode \
  -v jcode-state:/var/lib/jcode \
  -v "$PWD:/workspace" \
  -w /workspace \
  jcode:headless serve --server-name docker
```

No `EXPOSE` directive or TCP listener is required. The daemon communicates
over a local Unix socket inside the container. Operational output goes to
stdout/stderr and `$JCODE_HOME/logs/`. Outbound MCP or skill integrations
remain available when configured, and agent tools such as `gmail` can send
messages on the agent's behalf.

Upgrade a deployment by replacing and restarting the container. jcode does not
update or hot-reload its own executable.

## Build from source

```bash
cargo build --locked --release --bin jcode
cargo test --workspace
```

For a minimal feature build:

```bash
cargo build --locked --release --no-default-features --bin jcode
```

## Configuration

With `JCODE_HOME` set, configuration is read from
`$JCODE_HOME/config/jcode/config.toml`. Runtime state, sockets, sessions,
credentials, and logs stay under the same volume or the configured runtime
directory.

Useful headless integrations include lifecycle hooks, MCP servers, skills,
memory, and structured logs.

## License

[MIT](LICENSE)
