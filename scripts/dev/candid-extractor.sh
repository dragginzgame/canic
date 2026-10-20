#!/usr/bin/env bash
set -euo pipefail

[[ $# == 0 || ( $# == 1 && "$1" == --check ) ]] || {
    echo 'usage: candid-extractor.sh [--check]' >&2; exit 2;
}
root="${BASH_SOURCE[0]}"
[[ "$root" == /* ]] || root="$PWD/$root"
root="$(CDPATH='' cd -P "${root%/*}/../.." && pwd -P)"
# shellcheck source=tool-versions.env
source "$root/tool-versions.env"
# The shared installer owns receipts, build identity and retained installations.
# This adapter owns only Canic's package selection and returns the admitted path.
exec bash "$root/scripts/dev/install-rust-tools.sh" --consumer "$root" \
    --package candid-extractor --version "$CANIC_CANDID_EXTRACTOR_VERSION" \
    --bin candid-extractor --profile release "$@"
