#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
bash "$root/scripts/release/selection.sh"
tag="${1:?}" version="${2:?}"
[[ "$tag" == "$RELEASE_GROUP-v$version" ]] || exit 1
receipt="$(git -C "$root" show "$tag:releases/$RELEASE_GROUP/release-validation.json")"
policy="$(git -C "$root" rev-parse "$tag:ci/cargo-release-groups.json")"
admitted="$(jq -ser --arg version "$version" --arg group "$RELEASE_GROUP" --arg policy "$policy" '
    select(length == 1) | .[0] |
    select((.gate == "plan" and keys == ["date","gate","group","policy","qualification","schema","source","validation","version"]) or
        (.gate != "plan" and keys == ["date","gate","group","policy","schema","source","version"])) |
    select(.schema == 1 and .group == $group and .policy == $policy and .version == $version) |
    select(.source | type == "string" and test("^[0-9a-f]{40}$")) |
    select(.date | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$")) |
    select(.gate == "complete" or .gate == "fast" or (.gate == "plan" and
        (.validation.plan.scope == "workspace" or .validation.plan.scope == "group"))) |
    [.source, (if .gate == "plan" then "plan-" + .validation.plan.scope else .gate end)] | @tsv
' <<<"$receipt")"
if [[ "$(jq -r .gate <<<"$receipt")" == plan ]]; then
    mkdir -p "$root/target/tmp"
    proof="$(mktemp "$root/target/tmp/canic-published-validation.XXXXXX")"
    trap 'rm -f -- "$proof"' EXIT
    jq -cS .validation <<<"$receipt" > "$proof"
    [[ "$(jq -r .source <<<"$receipt")" == "$(jq -r .plan.source "$proof")" ]]
    perl "$root/scripts/release/validation-evidence.pl" historical "$proof"
fi
printf '%s\n' "$admitted"
