#!/usr/bin/env bash
set -euo pipefail

# Rewrite only selected-group records that the private consumer actually resolves
# from the exact root-owned manifest. Equal numeric versions confer no ownership.
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
[[ $# == 4 ]] || exit 2
names="$(jq -er --slurpfile plan "$1" --slurpfile owned "$2" '
    $owned[0].workspace_members as $members |
    [$owned[0].packages[] | select((.id as $id | $members | index($id)) and
        (.name as $n | $plan[0].members | index($n)))] as $owners |
    [.packages[] | select(.source == null) as $selected |
        $owners[] | select(.name == $selected.name and .version == $selected.version and
            .manifest_path == $selected.manifest_path) | .name] | unique | .[]
' "$3")" || { status=$?; [[ "$status" == 4 ]] || exit "$status"; names=''; }
if [[ -z "$names" ]]; then cat "$4"; exit; fi
packages=()
while IFS= read -r name; do packages+=("$name"); done <<< "$names"
editor="$(bash "$root/scripts/dev/release-metadata-editor.sh" --check)"
exec "$editor" --max-input-bytes 8388608 lock "$4" "${RELEASE_PREVIOUS:?}" "${RELEASE_VERSION:?}" "${packages[@]}"
