#!/usr/bin/env bash
set -euo pipefail

# Called after the index and candidate checks. Tag only the commit just created.
version="$(bash scripts/ci/read-workspace-version.sh)"
tag="v$version"
if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
    echo "Tag $tag already exists." >&2
    exit 1
fi
source="$(git rev-parse HEAD)"
git commit -m "Release $version"
release="$(git rev-parse HEAD)"
[[ "$release" != "$source" && "$(git rev-parse HEAD^)" == "$source" ]] || {
    echo "Release commit did not create one successor to the reviewed source." >&2
    exit 1
}
[[ "$(git log -1 --format=%s HEAD)" == "Release $version" ]] || {
    echo "Release commit identity differs from the reviewed version." >&2
    exit 1
}
git tag -a "$tag" "$release" -m "Release $version"
