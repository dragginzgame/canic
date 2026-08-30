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

release_entry_count="$(rg -c "^## ${planned//./\\.} - (Unreleased|[0-9]{4}-[0-9]{2}-[0-9]{2})$" "$detailed_changelog" || true)"
[[ "$release_entry_count" -eq 1 ]] || {
    echo "❌ $detailed_changelog must contain one $planned release entry (Unreleased or YYYY-MM-DD)." >&2
    exit 1
}

echo "✅ Release-notes preflight passed for $planned"
