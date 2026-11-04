#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
[[ $# -le 2 && ( -z "${2:-}" || "$2" == --check-remote ) ]] || exit 2
current="$(bash scripts/ci/read-workspace-version.sh)"
planned="$(bash scripts/ci/next-release-version.sh "$current" "${1:-patch}")"
changelog="$(bash scripts/release/selection.sh changelog)"
[[ -f "$changelog" ]] || { echo "Missing selected group ledger: $changelog" >&2; exit 1; }
scratch="$(mktemp "${TMPDIR:-/tmp}/canic-release-notes.XXXXXX")"
trap 'rm -f -- "$scratch"' EXIT
awk -v version="$planned" -v previous="$current" -v date="${RELEASE_DATE:-$(date -u +%F)}" \
    -f scripts/ci/finalize-release-changelog.awk "$changelog" > "$scratch"
if [[ "${2:-}" == --check-remote ]]; then
    bash scripts/ci/check-release-remote-state.sh before-version "$planned"
fi
printf 'Release notes admitted: %s %s\n' "$RELEASE_GROUP" "$planned"
