#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
source="${1:?}" version="${2:?}" date="${3:?}" commit="${4:-}"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-content.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$scratch"; else echo "Content comparison retained: $scratch" >&2; fi' EXIT
actual="$root"
if [[ -n "$commit" ]]; then
    mkdir "$scratch/actual"
    git archive "$commit" | tar -xf - -C "$scratch/actual"
    actual="$scratch/actual"
fi
receipt="releases/$RELEASE_GROUP/release-validation.json"
policy="$(git rev-parse "$source:ci/cargo-release-groups.json")"
gate="$(jq -er --arg group "$RELEASE_GROUP" --arg source "$source" --arg policy "$policy" \
    --arg version "$version" --arg date "$date" '
    select((.gate == "plan" and keys == ["date","gate","group","policy","qualification","schema","source","validation","version"]) or
        (.gate != "plan" and keys == ["date","gate","group","policy","schema","source","version"])) |
    select(.schema == 1 and .group == $group and .source == $source and .policy == $policy and
        .version == $version and .date == $date and (.gate == "complete" or .gate == "fast" or .gate == "plan" or .gate == "maintainer-waived")) | .gate
' "$actual/$receipt")"
mkdir "$scratch/source"
git archive "$source" | tar -xf - -C "$scratch/source"
export RELEASE_PREVIOUS RELEASE_VERSION="$version" RELEASE_DATE="$date" CANIC_RELEASE_VALIDATION_KIND="$gate"
if [[ "$gate" == plan ]]; then
    jq -cS .validation "$actual/$receipt" > "$scratch/validation.json"
    export CANIC_RELEASE_PLAN_EVIDENCE="$scratch/validation.json"
fi
RELEASE_PREVIOUS="$(cd "$scratch/source" && RELEASE_COMMIT='' bash "$root/scripts/ci/cargo-release-group.sh" version "$RELEASE_GROUP")"
mkdir "$scratch/expected"
bash scripts/release/render.sh "$source" "$scratch/expected"
# Every manifest and lock is compared, including unselected groups and private
# consumers. The selected notes retain editorial freedom; their dated identity
# is admitted separately, and exact delivery payloads remain runner-bound.
while IFS= read -r -d '' path; do
    relative="${path#"$scratch/expected/"}"
    cmp -- "$path" "$actual/$relative" || exit 1
done < <(find "$scratch/expected" -type f \( -name Cargo.toml -o -name Cargo.lock \) -print0)
for path in scripts/dev/install_dev.sh "$receipt"; do
    cmp -- "$scratch/expected/$path" "$actual/$path"
done
printf 'Exact selected-group transaction verified: %s %s\n' "$RELEASE_GROUP" "$version"
