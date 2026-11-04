#!/usr/bin/env bash
set -euo pipefail

# Product group selection precedes setup, Cargo observations and release effects.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
export PATH="$root/.tools/host/bin:$PATH"
[[ $# -le 1 ]] || exit 2
[[ -n "${RELEASE_GROUP:-}" ]] || {
    echo 'Select RELEASE_GROUP=framework, host, icydb or blob-service explicitly' >&2
    exit 2
}
selection="$(jq -ce --arg group "$RELEASE_GROUP" '
    [.groups[] | select(.name == $group)] |
    if length == 1 then .[0] else error("unknown RELEASE_GROUP") end
' "$root/ci/cargo-release-groups.json")" || exit 2
case "${1:-check}" in
    check) ;;
    primary) jq -er .primary <<<"$selection" ;;
    changelog) jq -er .changelog <<<"$selection" ;;
    receipt) printf 'releases/%s/release-validation.json\n' "$RELEASE_GROUP" ;;
    *) exit 2 ;;
esac
