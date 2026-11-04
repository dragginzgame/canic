#!/usr/bin/env bash
set -euo pipefail

# Reproduce the selected group's transaction from one immutable source. Shared
# owns Cargo rendering; Canic owns private consumer locks, CLI pin and receipt.
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$root/.tools/host/bin:$root/.tools/rust/common/bin:$PATH"
bash "$root/scripts/release/selection.sh"
[[ $# == 2 && "$1" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || exit 2
source="$1" output="$2"
[[ -d "$output" && -z "$(ls -A "$output")" ]] || exit 2
output="$(cd "$output" && pwd -P)"
git -C "$root" archive "$source" | tar -xf - -C "$output"
if [[ "${CANIC_RELEASE_VALIDATION_KIND:-complete}" == plan ]]; then
    proof="${CANIC_RELEASE_PLAN_EVIDENCE:-$root/target/release-validation/$RELEASE_GROUP/$source.json}"
    CANIC_RELEASE_VALIDATED_HEAD="$source" perl "$root/scripts/release/validation-evidence.pl" verify "$proof"
fi
policy="$(git -C "$root" rev-parse "$source:ci/cargo-release-groups.json")"
export CARGO_RELEASE_METADATA_EDITOR
CARGO_RELEASE_METADATA_EDITOR="$(bash "$root/scripts/dev/release-metadata-editor.sh" --check)"
export CARGO_RELEASE_METADATA_MAX_BYTES=8388608
export RELEASE_COMMIT=''
cd "$output"
evidence="$(mktemp -d "${TMPDIR:-/tmp}/canic-group-render.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$evidence"; else echo "Group render evidence retained: $evidence" >&2; fi' EXIT
bash "$root/scripts/ci/cargo-release-group.sh" plan "$RELEASE_GROUP" > "$evidence/plan.json"
primary="$(jq -er .primary "$evidence/plan.json")"
[[ "$(jq -er .previous "$evidence/plan.json")" == "${RELEASE_PREVIOUS:?}" ]]
cargo metadata --locked --offline --format-version 1 > "$evidence/root.json"
locks=(integrations/blob-service/consumer/Cargo.lock integrations/blob-service/embedded-consumer/Cargo.lock)
for index in "${!locks[@]}"; do
    lock="${locks[$index]}"
    cargo metadata --locked --offline --format-version 1 --manifest-path "${lock%/Cargo.lock}/Cargo.toml" > "$evidence/consumer-$index.json"
    cp -p "$lock" "$evidence/consumer-$index.lock"
done
bash "$root/scripts/ci/cargo-release-metadata.sh" prepare "$primary"
for index in "${!locks[@]}"; do
    lock="${locks[$index]}"
    bash "$root/scripts/release/rewrite-owned-lock.sh" "$evidence/plan.json" \
        "$evidence/root.json" "$evidence/consumer-$index.json" "$evidence/consumer-$index.lock" > "$evidence/candidate.lock"
    cat "$evidence/candidate.lock" > "$lock"
    cargo metadata --locked --offline --format-version 1 --manifest-path "${lock%/Cargo.lock}/Cargo.toml" > /dev/null
done
if [[ "$RELEASE_GROUP" == host ]]; then
    bash scripts/ci/sync-release-surface-version.sh "${RELEASE_VERSION:?}"
fi
# The exact policy blob and validated source are evidence, never handoff prose.
jq -n --arg group "$RELEASE_GROUP" --arg version "${RELEASE_VERSION:?}" \
    --arg source "$source" --arg policy "$policy" --arg date "${RELEASE_DATE:?}" \
    --arg gate "${CANIC_RELEASE_VALIDATION_KIND:-complete}" \
    '{schema:1,group:$group,version:$version,source:$source,policy:$policy,date:$date,gate:$gate}' \
    > "releases/$RELEASE_GROUP/release-validation.json"
if [[ "${CANIC_RELEASE_VALIDATION_KIND:-complete}" == plan ]]; then
    (cd "$root"; perl scripts/release/validation-evidence.pl project "$proof" "$output") > "$evidence/qualification.json"
    jq --slurpfile validation "$proof" --slurpfile qualification "$evidence/qualification.json" \
        '. + {validation:$validation[0],qualification:$qualification[0]}' \
        "releases/$RELEASE_GROUP/release-validation.json" > "$evidence/receipt.json"
    cat "$evidence/receipt.json" > "releases/$RELEASE_GROUP/release-validation.json"
fi
