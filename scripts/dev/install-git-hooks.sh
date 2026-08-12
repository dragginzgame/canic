#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOOK="$ROOT_DIR/.githooks/pre-commit"

if ! git -C "$ROOT_DIR" rev-parse --git-dir >/dev/null 2>&1; then
    echo "Canic Git hook setup skipped: $ROOT_DIR is not a Git checkout."
    exit 0
fi

if [ ! -f "$HOOK" ]; then
    echo "Canic Git hook setup failed: missing $HOOK" >&2
    exit 1
fi

chmod +x "$HOOK"
git -C "$ROOT_DIR" config --local core.hooksPath .githooks

echo "Canic formatting hook installed: .githooks/pre-commit"
