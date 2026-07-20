# syntax=docker/dockerfile:1.7
FROM rust:bookworm AS builder

ARG JCODE_FEATURES=""

RUN apt-get update \
    && apt-get install -y --no-install-recommends clang cmake pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN if [ -n "$JCODE_FEATURES" ]; then \
      cargo build --locked --release --no-default-features --features "$JCODE_FEATURES" --bin jcode; \
    else \
      cargo build --locked --release --no-default-features --bin jcode; \
    fi \
    && strip target/release/jcode

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      bash ca-certificates git openssh-client ripgrep tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /bin/bash jcode \
    && install -d -o jcode -g jcode /var/lib/jcode /workspace

COPY --from=builder /src/target/release/jcode /usr/local/bin/jcode

USER jcode
WORKDIR /workspace
ENV JCODE_HOME=/var/lib/jcode \
    JCODE_NON_INTERACTIVE=1 \
    RUST_BACKTRACE=1

VOLUME ["/var/lib/jcode"]
STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/bin/tini", "--", "jcode"]
CMD ["serve", "--server-name", "docker"]
