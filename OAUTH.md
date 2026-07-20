# Headless credential provisioning

jcode does not acquire credentials in the runtime process. There is no browser
callback listener, device-code flow, QR flow, or interactive login command.
Complete authentication on a trusted workstation and inject the resulting
credential into the container.

## Recommended model

Use a dedicated, writable `JCODE_HOME` volume. Import external credentials once
and let jcode own the durable copy:

| Provider | External source mounted for import | Writable native file |
| --- | --- | --- |
| Claude subscription | `$JCODE_HOME/external/.claude/.credentials.json` | `$JCODE_HOME/auth.json` |
| ChatGPT/Codex subscription | `$JCODE_HOME/external/.codex/auth.json` | `$JCODE_HOME/openai-auth.json` |

Pipe the corresponding JSON to `jcode auth import claude --stdin` or
`jcode auth import openai --stdin`. Alternatively, mount it at the external
path in the table and omit `--stdin`. Import is local, non-interactive, and
refuses a credential without a refresh token because that copy would not be
durable.

The external source may be mounted read-only. `JCODE_HOME` itself must remain
read-write because successful token refresh can rotate the refresh token.

## Direct native volume

If another trusted jcode installation already has working native credential
files, copy or mount its entire `JCODE_HOME` into the container. This is the
simplest deployment model and preserves account labels, refresh state, config,
and validation records together.

Treat the volume as a secret. Do not bake it into an image, commit it, print it
in logs, or pass token JSON through command-line arguments.

## API keys

API-key providers do not need an import step. Inject keys through the provider's
documented environment variable or a secret-backed env file under
`$JCODE_HOME/config/jcode`. API and subscription credentials can coexist.

## Validation

These commands never initiate login:

```bash
jcode auth status --json
jcode auth doctor claude --validate --json
jcode auth doctor openai --validate --json
```

If a credential is expired and cannot refresh, replace it from the trusted
workstation or repeat the explicit import with a newly authenticated source.
