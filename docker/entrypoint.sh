#!/bin/sh
# jcode container entrypoint.
#
# Goal: `docker run jcode run ...` executes with a background `jcode serve`
# daemon already up, so the container behaves like a live server box rather than
# a cold one-shot.
#
#   docker run jcode serve ...   -> exec the daemon directly (this container IS
#                                   the server); no second serve is started.
#   docker run jcode run ...     -> bring up a background daemon, wait until it
#                                   accepts connections, then exec `jcode run`.
#   docker run jcode <other>     -> exec directly (no daemon needed).
#
# The daemon's provider is fixed at startup, so when the `run` invocation carries
# `-p/--provider`, that value is parsed here and passed to the background
# `jcode serve` (as a global flag, before the subcommand). `-m/--model` needs no
# such handling: `jcode run` sets it on the attached session over the socket.
#
# A live server is detected with `jcode debug list`, which connects to the
# server's main socket and prints "running, debug: ..." only when it is actually
# accepting connections. The background daemon shares the container lifetime:
# tini (PID 1) reaps it and Docker tears it down when the foreground command
# exits.
set -e

JCODE_HOME="${JCODE_HOME:-/var/lib/jcode}"

# Route `jcode run` through the daemon started below so server-only features
# (swarm coordination) are available. Harmless outside `run`.
export JCODE_RUN_ATTACH_SERVER=1

server_is_up() {
    jcode debug list 2>/dev/null | grep -q 'running, debug'
}

# Extract the value of -p / --provider / --provider=X from the argument list,
# wherever it appears. Prints the provider (empty if none).
parse_provider() {
    take_next=0
    for arg in "$@"; do
        if [ "$take_next" = 1 ]; then
            printf '%s' "$arg"
            return 0
        fi
        case "$arg" in
            -p | --provider) take_next=1 ;;
            --provider=*) printf '%s' "${arg#--provider=}"; return 0 ;;
        esac
    done
}

ensure_serve() {
    provider="$1"
    if server_is_up; then
        return 0
    fi
    if [ -n "$provider" ]; then
        jcode -p "$provider" serve --server-name docker >"${JCODE_HOME}/serve.log" 2>&1 &
    else
        jcode serve --server-name docker >"${JCODE_HOME}/serve.log" 2>&1 &
    fi
    # Poll for readiness for up to ~10s (100 * 0.1s).
    i=0
    while [ "$i" -lt 100 ]; do
        if server_is_up; then
            return 0
        fi
        i=$((i + 1))
        sleep 0.1
    done
    echo "entrypoint: warning: jcode serve did not report ready within 10s; see ${JCODE_HOME}/serve.log" >&2
    return 0
}

case "$1" in
    run)
        ensure_serve "$(parse_provider "$@")"
        ;;
esac

exec jcode "$@"
