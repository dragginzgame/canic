#!/usr/bin/env bash
set -euo pipefail

# Read only the structured receipt frozen into an exact release tag.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TAG="${1:?usage: read-release-validation.sh <tag> <version>}"
VERSION="${2:?usage: read-release-validation.sh <tag> <version>}"

receipt="$(git -C "$ROOT" show "$TAG:release-validation.json")" || {
    echo "$TAG has no structured release validation receipt; use the complete lane" >&2
    exit 1
}
jq -ser --arg version "$VERSION" '
    select(length == 1) | .[0]
    | select(type == "object")
    | select(keys == ["date", "gate", "schema", "source", "version"])
    | select(.schema == 1 and .version == $version)
    | select(.source | type == "string" and test("^[0-9a-f]{40}$"))
    | select(.date | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))
    | select(.gate == "complete" or .gate == "fast")
    | [.source, .gate] | @tsv
' <<<"$receipt"
