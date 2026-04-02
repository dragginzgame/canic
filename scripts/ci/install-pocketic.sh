#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${POCKET_IC_VERSION:-}}"
if [ -z "$VERSION" ]; then
    echo "usage: install-pocketic.sh <version>" >&2
    exit 1
fi

TMP_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
DIR="$TMP_ROOT/pocket-ic-server-$VERSION"
BIN="$DIR/pocket-ic"

mkdir -p "$DIR"

if [ ! -x "$BIN" ]; then
    curl -L \
        -o "$BIN.gz" \
        "https://github.com/dfinity/pocketic/releases/download/$VERSION/pocket-ic-x86_64-linux.gz"
    gunzip -f "$BIN.gz"
    chmod +x "$BIN"
fi

printf '%s\n' "$BIN"
