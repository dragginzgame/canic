#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET_MIN=6
TARGET_MAX=10

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
if [ "$published_count" -lt "$TARGET_MIN" ]; then
    cadence_status="below the normal range"
elif [ "$published_count" -le "$TARGET_MAX" ]; then
    cadence_status="within the normal range"
else
    cadence_status="above the normal range"
fi

echo "Canic release cadence"
echo "  minor line: $minor_line"
echo "  published releases: $published_count"
echo "  normal planning range: $TARGET_MIN-$TARGET_MAX"
echo "  next release ordinal: $next_ordinal"
echo "  status: $cadence_status"

if [ "$next_ordinal" -gt "$TARGET_MAX" ]; then
    echo "  guidance: keep the current draft open for a complete release batch or record why another release boundary is necessary"
fi
