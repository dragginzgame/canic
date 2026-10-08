#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# The shared launcher owns stable server paths and executable selection.
export SCCACHE_BIN="${CANIC_SCCACHE_BIN:-}"
launcher="$ROOT/scripts/ci/run-sccache.sh"
if [[ $# -eq 0 || "$1" == -* ]]; then
    exec bash "$launcher" "$@"
fi

# sccache 0.17 reports its own failures with exit 2 and this diagnostic prefix.
# Compiler failures are forwarded separately, even when the compiler exits 2.
# Its built-in I/O fallback does not cover the initial server connection.
diagnostics="$(mktemp)"
trap 'rm -f -- "$diagnostics"' EXIT
status=0
bash "$launcher" "$@" 2>"$diagnostics" || status=$?
if [[ "$status" -eq 2 ]] && grep -q '^sccache: error:' "$diagnostics"; then
    # A working compiler fallback needs no per-crate warning. Opt in when
    # diagnosing cache availability; compiler diagnostics remain untouched.
    if [[ "${CANIC_SCCACHE_VERBOSE:-0}" == 1 ]]; then
        echo "sccache: warning: cache unavailable; running compiler directly" >&2
        sed 's/^sccache: error:/sccache: warning:/' "$diagnostics" >&2
    fi
    rm -f -- "$diagnostics"
    trap - EXIT
    exec "$@"
fi
cat "$diagnostics" >&2
exit "$status"
