#!/usr/bin/env bash
set -euo pipefail

# Canic chooses current versus committed source; the shared reader owns TOML/SemVer.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$ROOT/.tools/host/bin:$PATH"
[[ $# == 0 || ( $# == 1 && "$1" == --committed ) ]] || {
    echo 'usage: read-workspace-version.sh [--committed]' >&2; exit 2;
}
view="$ROOT"
scratch=""
trap '[[ -z "$scratch" ]] || rm -rf -- "$scratch"' EXIT
if [[ $# == 1 ]]; then
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/canic-committed-version.XXXXXX")"
    git -C "$ROOT" archive HEAD | tar -xf - -C "$scratch"
    view="$scratch"
fi
bash "$ROOT/scripts/ci/read-cargo-workspace-version.sh" "$view/Cargo.toml"
