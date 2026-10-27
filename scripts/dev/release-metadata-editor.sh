#!/usr/bin/env bash
# Shared companions: scripts/dev/install-rust-tools.sh tool-versions.env
set -euo pipefail
[[ $# == 0 || ( $# == 1 && "$1" == --check ) ]] || {
    echo 'usage: release-metadata-editor.sh [--check]' >&2; exit 2;
}
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
# shellcheck source=tool-versions.env
source "$root/tool-versions.env"
exec bash "$root/scripts/dev/install-rust-tools.sh" --consumer "$root" \
    --package ic-build --version "$CANIC_RELEASE_METADATA_VERSION" \
    --bin ic-build-release-metadata --profile debug --features cli "$@"
