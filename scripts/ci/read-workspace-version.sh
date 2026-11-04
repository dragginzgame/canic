#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
bash "$root/scripts/release/selection.sh"
[[ $# == 0 || ( $# == 1 && "$1" == --committed ) ]] || exit 2
cd "$root"
if [[ $# == 1 ]]; then
    export RELEASE_COMMIT
    RELEASE_COMMIT="$(git rev-parse HEAD)"
fi
exec bash "$root/scripts/ci/cargo-release-group.sh" version "$RELEASE_GROUP"
