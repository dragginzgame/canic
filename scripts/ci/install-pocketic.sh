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

bash "$SCRIPT_DIR/check-pocketic-version-alignment.sh" >&2

if [ -n "${RUNNER_TEMP:-}" ]; then
    CACHE_ROOT="$RUNNER_TEMP"
elif [ -n "${CANIC_POCKET_IC_CACHE_DIR:-}" ]; then
    CACHE_ROOT="$CANIC_POCKET_IC_CACHE_DIR"
else
    CACHE_ROOT="${XDG_CACHE_HOME:-${HOME:?HOME must be set}/.cache}/canic"
fi
platform="$(bash "$SCRIPT_DIR/pocketic-platform.sh")"
IFS=$'\t' read -r archive_name archive_digest binary_digest <<<"$platform"
DIR="$CACHE_ROOT/pocket-ic-server-$CANIC_POCKET_IC_VERSION-${archive_name%.gz}"
BIN="$DIR/pocket-ic"
ARCHIVE="$DIR/$archive_name"

mkdir -p "$DIR"

if [ -x "$BIN" ]; then
    bash "$SCRIPT_DIR/verify-file-checksum.sh" \
        sha256 "$binary_digest" "$BIN"
else
    tmp_bin="$BIN.part"
    trap 'rm -f "$ARCHIVE" "$tmp_bin"' EXIT
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL -o "$ARCHIVE" \
        "https://github.com/dfinity/pocketic/releases/download/$CANIC_POCKET_IC_VERSION/$archive_name"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" \
        sha256 "$archive_digest" "$ARCHIVE"
    gzip -dc "$ARCHIVE" >"$tmp_bin"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" \
        sha256 "$binary_digest" "$tmp_bin"
    mv "$tmp_bin" "$BIN"
    chmod +x "$BIN"
fi

printf '%s\n' "$BIN"
