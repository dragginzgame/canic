#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUMP_TYPE="${1:-patch}"

cd "$ROOT"
current="$(bash scripts/ci/read-workspace-version.sh)"
IFS=. read -r major minor patch <<<"${current%%[-+]*}"
case "$BUMP_TYPE" in
    patch)
        planned="$major.$minor.$((patch + 1))"
        ;;
    minor)
        planned="$major.$((minor + 1)).0"
        ;;
    major)
        planned="$((major + 1)).0.0"
        ;;
    *)
        echo "❌ Unsupported version bump: $BUMP_TYPE" >&2
        exit 2
        ;;
esac

detailed_changelog="docs/changelog/${planned%.*}.md"
status_document="docs/status/current.md"

[[ -f "$detailed_changelog" ]] || {
    echo "❌ Missing detailed changelog for planned release $planned: $detailed_changelog" >&2
    exit 1
}
[[ -f "$status_document" ]] || {
    echo "❌ Missing current status document: $status_document" >&2
    exit 1
}

draft_count="$(rg -c -F "## $planned - Unreleased" "$detailed_changelog" || true)"
[[ "$draft_count" -eq 1 ]] || {
    echo "❌ $detailed_changelog must contain exactly one open $planned draft." >&2
    exit 1
}

echo "✅ Release draft preflight passed for $planned"
