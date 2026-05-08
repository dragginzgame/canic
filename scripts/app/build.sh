#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ $# -eq 0 ]; then
  echo "usage: build.sh [canister_name]"
  exit 1
fi

CANISTER_NAME="$1"

copy_icp_build_output() {
  local wasm_gz_path="$1"

  if [ -z "${ICP_WASM_OUTPUT_PATH:-}" ]; then
    return 0
  fi

  local wasm_path="${wasm_gz_path%.gz}"
  if [ "$wasm_path" = "$wasm_gz_path" ] || [ ! -f "$wasm_path" ]; then
    echo "missing ICP wasm output source for $CANISTER_NAME: $wasm_path" >&2
    exit 1
  fi

  cp "$wasm_path" "$ICP_WASM_OUTPUT_PATH"
}

if [ -f "$ROOT_DIR/crates/canic-cli/Cargo.toml" ]; then
  cd "$ROOT_DIR"
  WASM_GZ_PATH="$(cargo run -q -p canic-cli --bin canic -- build "$CANISTER_NAME")"
  echo "$WASM_GZ_PATH"
  copy_icp_build_output "$WASM_GZ_PATH"
  exit 0
fi

if command -v canic >/dev/null 2>&1; then
  WASM_GZ_PATH="$(canic build "$CANISTER_NAME")"
  echo "$WASM_GZ_PATH"
  copy_icp_build_output "$WASM_GZ_PATH"
  exit 0
fi

echo "missing canic binary: install canic or run from a Canic workspace" >&2
exit 1
