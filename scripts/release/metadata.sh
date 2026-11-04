#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$root/.tools/host/bin:$root/.tools/rust/common/bin:$PATH"
bash "$root/scripts/release/selection.sh"
[[ $# == 1 ]] || exit 2
operation="$1"
cd "$root"
plan="$(bash scripts/ci/cargo-release-group.sh plan "$RELEASE_GROUP")"
primary="$(jq -er .primary <<<"$plan")"
files=()
while IFS= read -r -d '' path; do files+=("$path"); done < <(jq -jr '.files[] | . + "\u0000"' <<<"$plan")
case "$operation" in
    plan) printf '%s\n' "$plan"; exit ;;
    files) printf '%s\0' "${files[@]}"; exit ;;
    version) jq -er .previous <<<"$plan"; exit ;;
    packages)
        cargo metadata --locked --offline --no-deps --format-version 1 |
            jq -er --argjson plan "$plan" '.packages[] | select(.publish != [] and (.name as $n | $plan.members | index($n))) | .name'
        exit ;;
    check) bash scripts/ci/cargo-release-metadata.sh check "$primary" --manifest-order-grouped; exit ;;
    prepare) ;;
    *) exit 2 ;;
esac
[[ "${CANIC_RELEASE_VALIDATED:-}" == 1 && "${CANIC_RELEASE_VALIDATED_HEAD:?}" == "$(git rev-parse HEAD)" ]]
bash scripts/ci/check-release-source.sh
# shellcheck source=scripts/ci/release-metadata-transaction.sh
source "$root/scripts/ci/release-metadata-transaction.sh"
# shellcheck disable=SC2329 # Transaction callback.
render() {
    local original="$1" candidate="$2" tree path
    tree="$(mktemp -d "${TMPDIR:-/tmp}/canic-group-source.XXXXXX")" || return
    if ! bash "$root/scripts/release/render.sh" "$CANIC_RELEASE_VALIDATED_HEAD" "$tree"; then
        echo "Group source retained: $tree" >&2
        return 1
    fi
    for path in "${files[@]}"; do
        cmp -- "$original/$path" "$root/$path" || return
        cp -p -- "$tree/$path" "$candidate/$path" || return
    done
    rm -rf -- "$tree"
}
# shellcheck disable=SC2329 # Transaction callback.
check() {
    bash "$root/scripts/ci/cargo-release-metadata.sh" check "$primary" || return
    for consumer in consumer embedded-consumer; do
        cargo metadata --locked --offline --format-version 1 --manifest-path "integrations/blob-service/$consumer/Cargo.toml" > /dev/null || return
    done
}
release_metadata_prepare "${TMPDIR:-/tmp}/canic-group-transaction.XXXXXX" render check "${files[@]}"
