# syntax=docker/dockerfile:1.7
#
# jcode headless runtime image.
#
#   docker build -t jcode .
#   docker run --rm jcode version
#   docker run --rm -i jcode auth import claude --stdin < creds.json
#   docker run --rm -v "$PWD:/workspace" jcode run -p claude "explain this repo"
#   docker run -d  -v jcode-state:/var/lib/jcode --name jcode jcode        # daemon
#
# The image ships full default features (embeddings, pdf, bedrock), matching a
# native `cargo build --release`. Swarm coordination is compiled in
# unconditionally, so a single `jcode serve` (or one-shot `jcode run`) hosts the
# whole in-process swarm; no docker-in-docker or inter-node networking needed.
#
# Slim build (drops embeddings/pdf/bedrock, ~20-30 MB smaller binary):
#   docker build --build-arg JCODE_FEATURES=minimal -t jcode:slim .
# where JCODE_FEATURES is a comma-separated feature list passed to
# `--no-default-features --features`; use the literal `minimal` for none.

FROM rust:bookworm AS builder

# Extra/override cargo features. Empty => full default features.
# A non-empty value builds `--no-default-features --features "<value>"`;
# the sentinel `minimal` means no features at all.
ARG JCODE_FEATURES=""

RUN apt-get update \
    && apt-get install -y --no-install-recommends clang cmake pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

# Cache the cargo registry and target dir across builds. The compiled binary
# lives inside the (non-persisted) target cache mount, so copy it out to a
# normal path within the same RUN before the mount is unmounted.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target,sharing=locked \
    set -eux; \
    case "$JCODE_FEATURES" in \
      "")        cargo build --locked --release --bin jcode ;; \
      "minimal") cargo build --locked --release --no-default-features --bin jcode ;; \
      *)         cargo build --locked --release --no-default-features --features "$JCODE_FEATURES" --bin jcode ;; \
    esac; \
    strip target/release/jcode; \
    cp target/release/jcode /usr/local/bin/jcode

# Pre-fetch the embedding model so it is baked into the image and never
# downloaded at runtime. Lands in a read-only path outside the state volume;
# JCODE_MODELS_DIR (set below) points jcode at it.
FROM debian:bookworm-slim AS model
ARG MINILM_BASE=https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /models/all-MiniLM-L6-v2 \
    && curl -fSL "${MINILM_BASE}/onnx/model.onnx" -o /models/all-MiniLM-L6-v2/model.onnx \
    && curl -fSL "${MINILM_BASE}/tokenizer.json" -o /models/all-MiniLM-L6-v2/tokenizer.json

FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="jcode" \
      org.opencontainers.image.description="Headless coding-agent runtime with multi-model and swarm coordination" \
      org.opencontainers.image.source="https://github.com/anthropics/jcode"

# Runtime tools the agent shells out to: git/ssh for VCS, ripgrep for search,
# iproute2 for the `ip monitor` network-reconnect path, tini as PID 1 so tool
# subprocesses are reaped and SIGTERM is forwarded cleanly.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      bash ca-certificates git openssh-client ripgrep iproute2 tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /bin/bash jcode \
    && install -d -o jcode -g jcode /var/lib/jcode /workspace

COPY --from=builder /usr/local/bin/jcode /usr/local/bin/jcode
COPY --from=model /models /opt/jcode/models
COPY --chmod=0755 docker/entrypoint.sh /usr/local/bin/entrypoint.sh

USER jcode
WORKDIR /workspace
ENV JCODE_HOME=/var/lib/jcode \
    JCODE_MODELS_DIR=/opt/jcode/models \
    JCODE_NON_INTERACTIVE=1 \
    RUST_BACKTRACE=1

VOLUME ["/var/lib/jcode"]
STOPSIGNAL SIGTERM

# Liveness for `docker service` / swarm deploys running the `serve` daemon.
# `jcode debug list` connects to the server's main socket; the daemon reports
# "running, debug: ..." only when it is actually accepting connections. Harmless
# (always healthy-after-start-period) for one-shot `run` containers.
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD jcode debug list | grep -q 'running, debug' || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
CMD ["serve", "--server-name", "docker"]
