#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"

exec "$SCRIPT_DIR/canic_installer.sh" canic-install-root "${ROOT_CANISTER}"
