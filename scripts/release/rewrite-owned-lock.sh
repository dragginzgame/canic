#!/usr/bin/env bash
set -euo pipefail

# Cargo metadata selects Canic's roster; the shared transformer owns lock bytes.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
# shellcheck source=scripts/ci/require-jq.sh
source "$ROOT/scripts/ci/require-jq.sh"
require_jq
[[ $# == 4 || $# == 5 ]] || {
    echo 'usage: rewrite-owned-lock.sh METADATA LOCKFILE PREVIOUS CANDIDATE [CONSUMER_METADATA]' >&2; exit 2;
}
# shellcheck disable=SC2016 # jq variables are literal inputs.
names="$("$JQ_BIN" -er --arg previous "$3" '
    .workspace_members as $members
    | [.packages[] | select(.id as $id | $members | index($id))
       | select(.version == $previous) | .name] | sort
    | if length > 0 then .[] else error("no owned packages at previous version") end
' "$1")"
if [[ $# == 5 ]]; then
    # Project only exact root-owned dependencies selected by this private consumer.
    # shellcheck disable=SC2016 # jq variables are literal inputs.
    names="$("$JQ_BIN" -er --slurpfile owned "$1" --arg previous "$3" '
        $owned[0].workspace_members as $members
        | [$owned[0].packages[] | select(.id as $id | $members | index($id))
           | select(.version == $previous)] as $owners
        | [.packages[] | select(.source == null and .version == $previous) as $selected
           | $owners[] | select(.name == $selected.name and .manifest_path == $selected.manifest_path)
           | .name] | unique | sort
        | if length > 0 then .[] else error("no exact owned consumer dependencies") end
    ' "$5")"
fi
packages=()
while IFS= read -r name; do packages[${#packages[@]}]="$name"; done <<< "$names"
exec perl "$ROOT/scripts/ci/rewrite-local-lock-versions.pl" "$2" "$3" "$4" "${packages[@]}"
