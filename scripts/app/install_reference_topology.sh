#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"

if command -v canic-install-reference-topology >/dev/null 2>&1; then
    exec canic-install-reference-topology "${ROOT_CANISTER}"
fi

exec cargo run -q -p canic-installer --bin canic-install-reference-topology -- "${ROOT_CANISTER}"
