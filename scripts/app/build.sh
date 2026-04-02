#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ $# -eq 0 ]; then
  echo "usage: build.sh [canister_name]"
  exit 1
fi

CANISTER_NAME="$1"
RELEASE_VALUE="${RELEASE:-unset}"
DFX_NETWORK_VALUE="${DFX_NETWORK:-local}"
CANIC_WORKSPACE_ROOT_VALUE="${CANIC_WORKSPACE_ROOT:-$ROOT_DIR}"
CANIC_DFX_ROOT_VALUE="${CANIC_DFX_ROOT:-$CANIC_WORKSPACE_ROOT_VALUE}"

if [ "$RELEASE_VALUE" = "0" ]; then
  PROFILE_NAME="debug"
else
  PROFILE_NAME="wasm-release"
fi

echo "Canic build env: canister=${CANISTER_NAME} profile=${PROFILE_NAME} RELEASE=${RELEASE_VALUE} DFX_NETWORK=${DFX_NETWORK_VALUE} CANIC_WORKSPACE_ROOT=${CANIC_WORKSPACE_ROOT_VALUE} CANIC_DFX_ROOT=${CANIC_DFX_ROOT_VALUE}" >&2

exec "$SCRIPT_DIR/canic_installer.sh" canic-build-canister-artifact "$CANISTER_NAME"
