#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCCACHE_RUNTIME_ROOT="$ROOT/.tmp/sccache-runtime"
SCCACHE_RUNTIME_TMPDIR="$SCCACHE_RUNTIME_ROOT/tmp"
SCCACHE_BIN="${CANIC_SCCACHE_BIN:-}"

fail() {
    echo "sccache wrapper failed: $1" >&2
    exit 1
}

if [[ -z "$SCCACHE_BIN" ]]; then
    SCCACHE_BIN="$(command -v sccache 2>/dev/null || true)"
fi
[[ -n "$SCCACHE_BIN" && -x "$SCCACHE_BIN" ]] ||
    fail "the configured sccache executable is unavailable"

[[ ! -L "$ROOT/.tmp" ]] ||
    fail "repository scratch parent may not be a symlink"
mkdir -p "$SCCACHE_RUNTIME_TMPDIR" ||
    fail "cannot create the stable compiler-cache runtime directory"
[[ -d "$SCCACHE_RUNTIME_ROOT" && ! -L "$SCCACHE_RUNTIME_ROOT" ]] ||
    fail "compiler-cache runtime root must be a regular directory"
[[ -d "$SCCACHE_RUNTIME_TMPDIR" && ! -L "$SCCACHE_RUNTIME_TMPDIR" ]] ||
    fail "compiler-cache temporary path must be a regular directory"
chmod 700 "$SCCACHE_RUNTIME_ROOT" "$SCCACHE_RUNTIME_TMPDIR" ||
    fail "cannot protect the compiler-cache runtime directories"

# The sccache server outlives one test invocation. Never let it inherit the
# invocation-owned TMPDIR that test cleanup removes on exit.
export SCCACHE_SERVER_UDS="$SCCACHE_RUNTIME_ROOT/server.sock"
export TMPDIR="$SCCACHE_RUNTIME_TMPDIR"

# Cache-management commands have no underlying compiler to fall back to.
if [[ $# -eq 0 || "$1" == -* ]]; then
    exec "$SCCACHE_BIN" "$@"
fi

# sccache 0.17 reports its own failures with exit 2 and this diagnostic prefix.
# Compiler failures are forwarded separately, even when the compiler exits 2.
# Its built-in I/O fallback does not cover the initial server connection.
diagnostics="$(mktemp "$SCCACHE_RUNTIME_TMPDIR/client-error.XXXXXX")"
trap 'rm -f -- "$diagnostics"' EXIT
status=0
"$SCCACHE_BIN" "$@" 2>"$diagnostics" || status=$?
if [[ "$status" -eq 2 ]] && grep -q '^sccache: error:' "$diagnostics"; then
    echo "sccache: warning: cache unavailable; running compiler directly" >&2
    sed 's/^sccache: error:/sccache: warning:/' "$diagnostics" >&2
    rm -f -- "$diagnostics"
    trap - EXIT
    exec "$@"
fi
cat "$diagnostics" >&2
exit "$status"
