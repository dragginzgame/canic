#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

if [ "$#" -ne 0 ]; then
    echo "usage: install-pocketic.sh" >&2
    exit 1
fi

TMP_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
DIR="$TMP_ROOT/pocket-ic-server-$CANIC_POCKET_IC_VERSION"
BIN="$DIR/pocket-ic"
ARCHIVE="$DIR/pocket-ic-x86_64-linux.gz"

if [ "$(uname -s):$(uname -m)" != "Linux:x86_64" ] &&
    [ "$(uname -s):$(uname -m)" != "Linux:amd64" ]; then
    echo "unsupported PocketIC platform: $(uname -s) $(uname -m)" >&2
    exit 1
fi

mkdir -p "$DIR"

if [ -x "$BIN" ]; then
    bash "$SCRIPT_DIR/verify-file-checksum.sh" \
        sha256 "$CANIC_POCKET_IC_BINARY_SHA256_LINUX_X86_64" "$BIN"
else
    tmp_bin="$BIN.part"
    trap 'rm -f "$ARCHIVE" "$tmp_bin"' EXIT
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL -o "$ARCHIVE" \
        "https://github.com/dfinity/pocketic/releases/download/$CANIC_POCKET_IC_VERSION/pocket-ic-x86_64-linux.gz"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" \
        sha256 "$CANIC_POCKET_IC_ARCHIVE_SHA256_LINUX_X86_64" "$ARCHIVE"
    gzip -dc "$ARCHIVE" >"$tmp_bin"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" \
        sha256 "$CANIC_POCKET_IC_BINARY_SHA256_LINUX_X86_64" "$tmp_bin"
    mv "$tmp_bin" "$BIN"
    chmod +x "$BIN"
fi

printf '%s\n' "$BIN"
