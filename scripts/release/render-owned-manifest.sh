#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
# shellcheck source=scripts/ci/require-jq.sh
source "$ROOT/scripts/ci/require-jq.sh"
require_jq
[[ $# == 4 ]] || {
    echo 'usage: render-owned-manifest.sh METADATA MANIFEST PREVIOUS CANDIDATE' >&2; exit 2;
}
editor="$(bash "$ROOT/scripts/dev/release-metadata-editor.sh" --check)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/canic-manifest-render.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf "$scratch"; else printf "Manifest rendering evidence retained: %s\n" "$scratch" >&2; fi' EXIT
yq -p toml -o json '.' "$2" > "$scratch/manifest.json"
"$JQ_BIN" -n --slurpfile manifest "$scratch/manifest.json" --slurpfile metadata "$1" '
    {manifest: $manifest[0], members: ($metadata[0] | . as $m |
        [.packages[] | select(.id as $id | $m.workspace_members | index($id)) |
            {name, manifest_path}])}
' > "$scratch/workspace-input.json"
exec_status=0
"$editor" --max-input-bytes 8388608 manifest "$2" "$3" "$4" \
    "$scratch/workspace-input.json" || exec_status=$?
exit "$exec_status"
