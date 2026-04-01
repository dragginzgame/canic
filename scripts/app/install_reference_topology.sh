#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"

cargo run -q -p canic-internal --bin install_reference_topology -- "${ROOT_CANISTER}"
