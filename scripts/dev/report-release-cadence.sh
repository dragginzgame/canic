#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MAX_RELEASES_PER_MINOR=12

if [ "$#" -gt 1 ]; then
    echo "usage: scripts/dev/report-release-cadence.sh [VERSION]" >&2
    exit 2
fi

cd "$ROOT"
version="${1:-$(cargo get workspace.package.version)}"
if [[ ! "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    echo "release cadence report requires a semantic workspace version, got: $version" >&2
    exit 1
fi

major="${BASH_REMATCH[1]}"
minor="${BASH_REMATCH[2]}"
minor_line="$major.$minor"
published_count=0
while IFS= read -r tag; do
    if [[ "$tag" =~ ^v${major}\.${minor}\.[0-9]+$ ]]; then
        published_count=$((published_count + 1))
    fi
done < <(git tag --list "v$minor_line.*")

next_ordinal=$((published_count + 1))
if [ "$next_ordinal" -le "$MAX_RELEASES_PER_MINOR" ]; then
    cadence_status="within guideline"
else
    cadence_status="next release exceeds guideline"
fi

echo "Canic release cadence"
echo "  minor line: $minor_line"
echo "  published releases: $published_count"
echo "  guideline: no more than $MAX_RELEASES_PER_MINOR releases per minor"
echo "  next release ordinal: $next_ordinal"
echo "  status: $cadence_status"

if [ "$next_ordinal" -gt "$MAX_RELEASES_PER_MINOR" ]; then
    echo "  guidance: keep the current draft open for a complete release batch or record why another release boundary is necessary"
fi
