#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ $# -eq 0 ]; then
  echo "usage: build.sh [canister_name]"
  exit 1
fi

CANISTER_NAME="$1"

if [ -f "$ROOT_DIR/crates/canic-cli/Cargo.toml" ]; then
  cd "$ROOT_DIR"
  exec cargo run -q -p canic-cli --bin canic -- build "$CANISTER_NAME"
fi

if command -v canic >/dev/null 2>&1; then
  exec canic build "$CANISTER_NAME"
fi

echo "missing canic binary: install canic or run from a Canic workspace" >&2
exit 1
