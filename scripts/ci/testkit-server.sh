#!/usr/bin/env bash
# Select the locked published runner through Shared Tooling's receipt check.
set -euo pipefail
if [[ "$#" -gt 1 || ( "$#" -eq 1 && "$1" != --check ) ]]; then
    echo 'usage: testkit-server.sh [--check]' >&2
    exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
exec bash "$ROOT/scripts/dev/install-rust-tools.sh" --consumer "$ROOT" \
    --package ic-testkit --lockfile "$ROOT/Cargo.lock" \
    --bin ic-testkit-server --profile release "$@"
