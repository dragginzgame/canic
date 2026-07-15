#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

fail() {
    echo "release push readiness check failed: $1" >&2
    exit 1
}

workspace_version() {
    awk '
        /^\[workspace.package\]/ { in_section = 1; next }
        /^\[/ && in_section { exit }
        in_section && $1 == "version" {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' Cargo.toml
}

version="$(workspace_version)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]] ||
    fail "Cargo.toml does not declare a valid workspace release version"

if [ -n "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    fail "the worktree or index is not clean"
fi

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

echo "✅ Release push is ready: $branch at $head_commit with $tag"
