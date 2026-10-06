#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 2 || ! "$1" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ||
    ! "$2" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo 'usage: check-release-tag.sh <exact-commit> <version>' >&2
    exit 2
fi
commit="$1"
version="$2"
[[ "$(git rev-parse --verify "$commit^{commit}")" == "$commit" ]]
tag="refs/tags/v$version"
[[ "$(git cat-file -t "$tag")" == tag ]] || {
    echo "release tag v$version must be annotated" >&2; exit 1;
}
[[ "$(git rev-parse --verify "$tag^{commit}")" == "$commit" ]] || {
    echo "release tag v$version does not point to $commit" >&2; exit 1;
}
