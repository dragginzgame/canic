#!/usr/bin/env bash
# shellcheck disable=SC2123 # Each subshell deliberately excludes host tool directories.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=scripts/ci/require-jq.sh
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-jq-selection.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
real_bash="$(command -v bash)"
mkdir -p "$fixture/path" "$fixture/empty" "$fixture/custom tools"
mkdir -p "$fixture/scripts/ci"
cp "$ROOT/scripts/ci/require-jq.sh" "$fixture/scripts/ci/"
source "$fixture/scripts/ci/require-jq.sh"
for executable in path/jq 'custom tools/jq' fallback; do
    printf '#!/bin/sh\nprintf "%%s\\n" "$0"\n' >"$fixture/$executable"
    chmod +x "$fixture/$executable"
done

(
    unset JQ_BIN
    PATH="$fixture/path"
    [[ "$(resolve_jq_executable "$fixture/fallback")" == "$fixture/path/jq" ]]
    PATH="$fixture/empty"
    [[ "$(resolve_jq_executable "$fixture/fallback")" == "$fixture/fallback" ]]
)
(
    PATH="$fixture/path"
    JQ_BIN="$fixture/custom tools/jq"
    require_jq
    [[ "$("$real_bash" -c 'cd /; "$JQ_BIN"')" == "$fixture/custom tools/jq" ]]
    for JQ_BIN in "$fixture/missing" "$fixture/custom tools"; do
        if resolve_jq_executable "$fixture/fallback" >/dev/null 2>&1; then
            echo 'invalid explicit jq selection silently fell back' >&2
            exit 1
        fi
    done
)
(
    cd "$fixture"
    unset JQ_BIN
    PATH=path
    selected="$(resolve_jq_executable "$fixture/fallback")"
    cd /
    [[ "$("$selected")" == "$fixture/path/jq" ]]
)
chmod -x "$fixture/fallback"
(
    unset JQ_BIN
    PATH="$fixture/empty"
    if resolve_jq_executable "$fixture/fallback" >/dev/null 2>&1; then
        echo 'nonexecutable jq fallback was accepted' >&2
        exit 1
    fi
)
mkdir -p "$fixture/.tools/host/bin"
cp "$fixture/path/jq" "$fixture/.tools/host/bin/jq"
(
    unset JQ_BIN
    PATH="$fixture/path"
    [[ "$(resolve_jq_executable "$fixture/fallback")" == "$fixture/.tools/host/bin/jq" ]]
    JQ_BIN="$fixture/custom tools/jq"
    [[ "$(resolve_jq_executable "$fixture/fallback")" == "$JQ_BIN" ]]
)
echo 'jq selection: explicit path, prepared repository tool, PATH, fallback and refusal passed'
