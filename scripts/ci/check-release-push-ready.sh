#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
VERSION_READER="$ROOT_DIR/scripts/ci/read-workspace-version.sh"
cd "$ROOT_DIR"

fail() {
    echo "release push readiness check failed: $1" >&2
    exit 1
}

version="$(bash "$VERSION_READER" --committed)" ||
    fail "cargo-get could not read the committed workspace version"

branch="$(git symbolic-ref --quiet --short HEAD)" ||
    fail "HEAD is detached"
subject="$(git log -1 --format=%s HEAD)"
[ "$subject" = "Release $version" ] ||
    fail "HEAD is not the Release $version commit"

tag="v$version"
tag_type="$(git cat-file -t "refs/tags/$tag" 2>/dev/null)" ||
    fail "annotated tag $tag is missing"
[ "$tag_type" = "tag" ] || fail "$tag is not an annotated tag"

head_commit="$(git rev-parse HEAD)"
tag_commit="$(git rev-list -n 1 "$tag")"
[ "$tag_commit" = "$head_commit" ] ||
    fail "$tag does not identify HEAD"

bash scripts/ci/check-release-remote-state.sh before-push "$version" ||
    fail "remote branch or release tag is no longer push-compatible"

echo "✅ Release push is ready: $branch at $head_commit with $tag"
