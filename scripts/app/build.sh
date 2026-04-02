#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [ $# -eq 0 ]; then
  echo "usage: build.sh [canister_name]"
  exit 1
fi

exec "$SCRIPT_DIR/canic_installer.sh" canic-build-canister-artifact "$1"
