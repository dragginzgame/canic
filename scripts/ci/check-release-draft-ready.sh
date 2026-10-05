#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUMP_TYPE="${1:-patch}"
CHECK_REMOTE="${2:-}"

if [[ $# -gt 2 || ( -n "$CHECK_REMOTE" && "$CHECK_REMOTE" != --check-remote ) ]]; then
    echo "usage: $0 [patch|minor|major] [--check-remote]" >&2
    exit 2
fi

cd "$ROOT"
current="$(bash scripts/ci/read-workspace-version.sh)"
planned="$(bash scripts/ci/next-release-version.sh "$current" "$BUMP_TYPE")"

detailed_changelog="docs/changelog/${planned%.*}.md"


scratch="$(mktemp "${TMPDIR:-/tmp}/canic-release-notes.XXXXXX")"
trap 'rm -f "$scratch"' EXIT
if [[ -f "$detailed_changelog" ]]; then
    awk -v version="$planned" -v date="${RELEASE_DATE:-$(date -u +%F)}" \
        -f scripts/ci/finalize-release-changelog.awk "$detailed_changelog" > "$scratch"
fi
awk -v version="$planned" -v date="${RELEASE_DATE:-$(date -u +%F)}" \
  -f scripts/ci/finalize-release-changelog.awk CHANGELOG.md > "$scratch"

if [[ "$CHECK_REMOTE" == --check-remote ]]; then
    bash scripts/ci/check-release-remote-state.sh before-version "$planned"
fi

echo "✅ Release-notes preflight passed for $planned"
