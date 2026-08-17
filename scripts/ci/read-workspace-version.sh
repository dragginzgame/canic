#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
read_committed=0

if [ "$#" -gt 1 ]; then
    echo "usage: scripts/ci/read-workspace-version.sh [--committed]" >&2
    exit 2
fi
if [ "$#" -eq 1 ]; then
    [ "$1" = "--committed" ] || {
        echo "usage: scripts/ci/read-workspace-version.sh [--committed]" >&2
        exit 2
    }
    read_committed=1
fi

if ! cargo get --version >/dev/null 2>&1; then
    echo "cargo-get is required to read the workspace version" >&2
    exit 1
fi

entry="$ROOT"
scratch=""
cleanup() {
    if [ -n "$scratch" ]; then
        rm -f -- "$scratch/Cargo.toml"
        rmdir -- "$scratch"
    fi
}
trap cleanup EXIT

if [ "$read_committed" -eq 1 ]; then
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/canic-workspace-version.XXXXXX")"
    (cd "$ROOT" && git show HEAD:Cargo.toml) >"$scratch/Cargo.toml" || {
        echo "HEAD does not contain Cargo.toml" >&2
        exit 1
    }
    entry="$scratch"
fi

version="$(cargo get --entry "$entry" workspace.package.version)" || {
    echo "failed to read the workspace version with cargo-get" >&2
    exit 1
}
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]] || {
    echo "workspace package version is not valid SemVer: $version" >&2
    exit 1
}

printf '%s\n' "$version"
