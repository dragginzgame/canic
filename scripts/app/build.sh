#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ $# -eq 0 ]; then
  echo "usage: build.sh [canister_name]"
  exit 1
fi

CANISTER_NAME="$1"
CANIC_WASM_PROFILE_VALUE="${CANIC_WASM_PROFILE:-unset}"
DFX_NETWORK_VALUE="${DFX_NETWORK:-local}"
CANIC_WORKSPACE_ROOT_VALUE="${CANIC_WORKSPACE_ROOT:-$ROOT_DIR}"
CANIC_DFX_ROOT_VALUE="${CANIC_DFX_ROOT:-$CANIC_WORKSPACE_ROOT_VALUE}"

if [ "$CANIC_WASM_PROFILE_VALUE" = "debug" ]; then
  PROFILE_NAME="debug"
elif [ "$CANIC_WASM_PROFILE_VALUE" = "fast" ]; then
  PROFILE_NAME="fast"
elif [ "$CANIC_WASM_PROFILE_VALUE" = "release" ]; then
  PROFILE_NAME="release"
elif [ "$CANIC_WASM_PROFILE_VALUE" = "unset" ]; then
  PROFILE_NAME="release"
else
  echo "invalid CANIC_WASM_PROFILE=${CANIC_WASM_PROFILE_VALUE}; expected debug, fast, or release" >&2
  exit 1
fi

echo "Canic build env: canister=${CANISTER_NAME} profile=${PROFILE_NAME} CANIC_WASM_PROFILE=${CANIC_WASM_PROFILE_VALUE} DFX_NETWORK=${DFX_NETWORK_VALUE} CANIC_WORKSPACE_ROOT=${CANIC_WORKSPACE_ROOT_VALUE} CANIC_DFX_ROOT=${CANIC_DFX_ROOT_VALUE}" >&2

exec "$SCRIPT_DIR/canic_installer.sh" canic-build-canister-artifact "$CANISTER_NAME"
