#!/usr/bin/env bash
set -euo pipefail

if [[ "${CANIC_RELEASE_PUSH_READY:-}" != "1" ]]; then
    echo "release push refused without the verified Make release boundary" >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"
cd "$ROOT" || exit 1

workspace_version="$(bash "$VERSION_READER" --committed)"
branch="$(git symbolic-ref --quiet --short HEAD)"
tag="v$workspace_version"

git push --no-follow-tags --atomic origin \
    "HEAD:refs/heads/$branch" \
    "refs/tags/$tag:refs/tags/$tag"
