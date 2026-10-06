#!/usr/bin/env bash

_CANIC_REQUIRE_JQ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"

# Resolve one executable without changing PATH or the caller's shell settings.
resolve_jq_executable() {
    local fallback="$1"
    local selected="${JQ_BIN:-}"
    if [[ -z "$selected" ]]; then
        if [[ -x "$_CANIC_REQUIRE_JQ_ROOT/.tools/host/bin/jq" ]]; then
            selected="$_CANIC_REQUIRE_JQ_ROOT/.tools/host/bin/jq"
        fi
    fi
    if [[ -z "$selected" ]]; then
        selected="$(command -v jq)" || selected="$fallback"
    fi
    if [[ ! -f "$selected" || ! -x "$selected" ]]; then
        printf 'jq is unavailable at %s; install jq or set JQ_BIN to its executable\n' \
            "$selected" >&2
        return 1
    fi
    # Retain the same executable if a release helper later changes directory.
    case "$selected" in
        /*) printf '%s\n' "$selected" ;;
        *) printf '%s/%s\n' "$PWD" "$selected" ;;
    esac
}

require_jq() {
    JQ_BIN="$(resolve_jq_executable "$HOME/.local/bin/jq")" || return 1
    export JQ_BIN
}
