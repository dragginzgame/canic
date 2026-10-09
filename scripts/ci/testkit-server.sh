#!/usr/bin/env bash
# Resolve the selected published runner through Shared Tooling's receipt check.
set -euo pipefail
if [[ "$#" -gt 1 || ( "$#" -eq 1 && "$1" != --check ) ]]; then
    echo 'usage: testkit-server.sh [--check]' >&2
    exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck source=/dev/null
source "$ROOT/tool-versions.env"
exec bash "$ROOT/scripts/dev/install-rust-tools.sh" --consumer "$ROOT" \
    --package ic-testkit --version "$CANIC_TESTKIT_SERVER_VERSION" \
    --bin ic-testkit-server --profile release "$@"
