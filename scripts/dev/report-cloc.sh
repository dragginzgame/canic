#!/usr/bin/env bash
set -euo pipefail

# Canic selects its JSON tool; the canonical reporter owns Cargo/source counting.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck source=scripts/ci/require-jq.sh
source "$ROOT/scripts/ci/require-jq.sh"
require_jq
view="$(mktemp -d "${TMPDIR:-/tmp}/canic-cloc-tools.XXXXXX")"
trap 'rm -rf -- "$view"' EXIT
ln -s "$JQ_BIN" "$view/jq"
if [[ $# == 0 ]]; then set -- "$ROOT"; fi
PATH="$view:$PATH" bash "$ROOT/scripts/dev/cloc.sh" "$@"
