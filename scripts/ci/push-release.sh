#!/usr/bin/env bash
set -euo pipefail

if [[ "${CANIC_RELEASE_PUSH_READY:-}" != "1" ]]; then
    echo "release push refused without the verified Make release boundary" >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT" || exit 1

workspace_version="$(awk '
    /^\[workspace.package\]/ { in_section = 1; next }
    /^\[/ && in_section { exit }
    in_section && $1 == "version" {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' Cargo.toml)"
branch="$(git symbolic-ref --quiet --short HEAD)"
tag="v$workspace_version"

git push --atomic origin \
    "HEAD:refs/heads/$branch" \
    "refs/tags/$tag:refs/tags/$tag"
