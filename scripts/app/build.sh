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
BUILD_CONTEXT_MARKER_DIR="$CANIC_DFX_ROOT_VALUE/.dfx"
BUILD_CONTEXT_MARKER_FILE="$BUILD_CONTEXT_MARKER_DIR/.canic-build-context-${PPID}"

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

mkdir -p "$BUILD_CONTEXT_MARKER_DIR"

if [ ! -e "$BUILD_CONTEXT_MARKER_FILE" ]; then
  : > "$BUILD_CONTEXT_MARKER_FILE"
  echo "Canic build context: profile=${PROFILE_NAME} requested_profile=${CANIC_WASM_PROFILE_VALUE} DFX_NETWORK=${DFX_NETWORK_VALUE} CANIC_WORKSPACE_ROOT=${CANIC_WORKSPACE_ROOT_VALUE} CANIC_DFX_ROOT=${CANIC_DFX_ROOT_VALUE}" >&2
fi

echo >&2
echo "Canic build start: canister=${CANISTER_NAME} profile=${PROFILE_NAME}" >&2

BUILD_STARTED_AT="${EPOCHREALTIME}"
"$SCRIPT_DIR/canic_installer.sh" canic-build-canister-artifact "$CANISTER_NAME"
BUILD_FINISHED_AT="${EPOCHREALTIME}"
ELAPSED_SECONDS="$(awk -v start="$BUILD_STARTED_AT" -v end="$BUILD_FINISHED_AT" 'BEGIN { printf "%.2f", (end - start) }')"

echo "Canic build done: canister=${CANISTER_NAME} elapsed=${ELAPSED_SECONDS}s" >&2
