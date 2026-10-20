#!/usr/bin/env bash
set -euo pipefail

# Verify canonical payload ownership, then links in their actual consumer layout.
# Consumer-owned targets such as README.md are intentionally outside the snapshot.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh"
documents=()
while IFS= read -r path; do
    [[ -n "$path" ]] || { echo 'empty governance selection' >&2; exit 1; }
    awk -F '\t' -v selected="$path" '
        $1 == "file" && $4 == selected { found = 1 }
        END { exit !found }
    ' "$ROOT/.shared-tooling.snapshot" || {
        echo "governance input is outside the snapshot: $path" >&2; exit 1;
    }
    case "$path" in *.md) documents[${#documents[@]}]="$path" ;; esac
done < "$ROOT/scripts/distribution/governance-files.txt"
[[ ${#documents[@]} -gt 0 ]] || { echo 'governance export has no documents' >&2; exit 1; }
perl "$ROOT/scripts/ci/check-documentation-links.pl" --root "$ROOT" "${documents[@]}"
