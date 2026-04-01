#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"

cargo run -q -p canic-installer --bin canic-install-reference-topology -- "${ROOT_CANISTER}"
