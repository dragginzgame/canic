#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN_NAME="${1:-}"

if [ -z "$BIN_NAME" ]; then
    echo "usage: canic_installer.sh <canic-installer-binary> [args...]" >&2
    exit 1
fi
shift

case "$BIN_NAME" in
    canic-*) ;;
    *)
        echo "unsupported canic-installer binary: $BIN_NAME" >&2
        exit 1
        ;;
esac

if [ -f "$ROOT_DIR/crates/canic-installer/Cargo.toml" ]; then
    cd "$ROOT_DIR"
    exec cargo run -q -p canic-installer --bin "$BIN_NAME" -- "$@"
fi

if command -v "$BIN_NAME" >/dev/null 2>&1; then
    exec "$BIN_NAME" "$@"
fi

echo "missing canic-installer binary '$BIN_NAME': no local workspace crate and nothing installed on PATH" >&2
exit 1
