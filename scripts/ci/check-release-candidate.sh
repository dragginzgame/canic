#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
commit=''
if [[ $# != 0 ]]; then
    [[ $# == 2 && "$1" == --commit && "$2" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || exit 2
    commit="$2"
fi
export RELEASE_COMMIT="$commit"
version="$(bash scripts/ci/read-workspace-version.sh)"
[[ -z "${RELEASE_VERSION:-}" || "$RELEASE_VERSION" == "$version" ]]
subject="Release $RELEASE_GROUP $version"
actual_subject="$(git log -1 --format=%s "${commit:-HEAD}")"
if [[ "$actual_subject" == "$subject" ]]; then
    source="$(git rev-parse "${commit:-HEAD}^")"
else
    [[ -z "$commit" ]] || { echo 'Selected commit is not the group release' >&2; exit 1; }
    source="$(git rev-parse HEAD)"
fi
receipt="releases/$RELEASE_GROUP/release-validation.json"
if [[ -n "$commit" ]]; then record="$(git show "$commit:$receipt")"; else record="$(cat "$receipt")"; fi
export RELEASE_DATE RELEASE_VERSION="$version"
RELEASE_DATE="$(jq -er .date <<<"$record")"
files="$(mktemp "${TMPDIR:-/tmp}/canic-release-roster.XXXXXX")"
trap 'rm -f -- "$files"' EXIT
bash scripts/release/metadata.sh files > "$files"
paths=()
while IFS= read -r -d '' path; do paths+=(--allow "$path"); done < "$files"
if [[ -z "$commit" ]]; then
    bash scripts/ci/check-release-source.sh "${paths[@]}"
else
    while IFS= read -r changed; do
        admitted=false
        while IFS= read -r -d '' path; do [[ "$changed" != "$path" ]] || admitted=true; done < "$files"
        [[ "$admitted" == true ]] || { echo "Non-release change: $changed" >&2; exit 1; }
    done < <(git diff --name-only "$source" "$commit")
fi
bash scripts/release/metadata.sh check
bash scripts/ci/check-release-surface-content.sh "$source" "$version" "$RELEASE_DATE" "$commit"
if [[ "$RELEASE_GROUP" == host ]]; then
    if [[ -n "$commit" ]]; then installer="$(git show "$commit:scripts/dev/install_dev.sh")"; else installer="$(cat scripts/dev/install_dev.sh)"; fi
    [[ "$(rg -c -F "CANIC_CLI_VERSION=\"\${CANIC_CLI_VERSION:-$version}\"" <<<"$installer")" == 1 ]]
fi
printf 'Release candidate admitted: %s %s\n' "$RELEASE_GROUP" "$version"
