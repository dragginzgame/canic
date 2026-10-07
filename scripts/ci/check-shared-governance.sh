#!/usr/bin/env bash
set -euo pipefail

# Canic selects the canonical governance export; shared helpers verify its inputs.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh"
view="$(mktemp -d "${TMPDIR:-/tmp}/canic-governance-export.XXXXXX")"
finish() {
    local status=$?
    if [[ "$status" == 0 ]]; then rm -rf -- "$view"
    else echo "Governance export evidence retained: $view" >&2; fi
}
trap finish EXIT
documents=()
while IFS= read -r path; do
    [[ -n "$path" ]] || { echo 'empty governance selection' >&2; exit 1; }
    awk -F '\t' -v selected="$path" '
        $1 == "file" && $4 == selected { found = 1 }
        END { exit !found }
    ' "$ROOT/.shared-tooling.snapshot" || {
        echo "governance input is outside the snapshot: $path" >&2; exit 1;
    }
    mkdir -p "$view/$(dirname "$path")"
    cp -p "$ROOT/$path" "$view/$path"
    case "$path" in *.md) documents[${#documents[@]}]="$path" ;; esac
done < "$ROOT/scripts/distribution/governance-files.txt"
[[ ${#documents[@]} -gt 0 ]] || { echo 'governance export has no documents' >&2; exit 1; }
perl "$ROOT/scripts/ci/check-documentation-links.pl" --root "$view" "${documents[@]}"
