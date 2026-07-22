#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

INSTALL_DIR="${IC_WASM_INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=""

resolve_platform() {
    case "$(uname -s):$(uname -m)" in
    Darwin:arm64 | Darwin:aarch64)
        archive_platform="aarch64-apple-darwin"
        checksum="$CANIC_IC_WASM_SHA256_DARWIN_ARM64"
        ;;
    Darwin:x86_64 | Darwin:amd64)
        archive_platform="x86_64-apple-darwin"
        checksum="$CANIC_IC_WASM_SHA256_DARWIN_X64"
        ;;
    Linux:arm64 | Linux:aarch64)
        archive_platform="aarch64-unknown-linux-gnu"
        checksum="$CANIC_IC_WASM_SHA256_LINUX_ARM64"
        ;;
    Linux:x86_64 | Linux:amd64)
        archive_platform="x86_64-unknown-linux-gnu"
        checksum="$CANIC_IC_WASM_SHA256_LINUX_X64"
        ;;
    *)
        echo "unsupported ic-wasm platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local package="ic-wasm-${archive_platform}"
    local archive="${package}.tar.xz"
    local url="https://github.com/dfinity/ic-wasm/releases/download/${CANIC_IC_WASM_VERSION}/${archive}"
    local installed
    local candidate
    local version_output

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$checksum" "$TMP_DIR/$archive"
    tar -xJf "$TMP_DIR/$archive" -C "$TMP_DIR" "$package/ic-wasm"

    candidate="$TMP_DIR/$package/ic-wasm"
    chmod +x "$candidate"
    version_output="$("$candidate" --version 2>&1)"
    case "$version_output" in
    *"$CANIC_IC_WASM_VERSION"*) ;;
    *)
        echo "installed ic-wasm does not report the pinned version" >&2
        echo "expected: $CANIC_IC_WASM_VERSION" >&2
        echo "actual:   $version_output" >&2
        exit 1
        ;;
    esac

    mkdir -p "$INSTALL_DIR"
    installed="$INSTALL_DIR/ic-wasm"
    mv "$candidate" "$installed"
    if [ -n "${GITHUB_PATH:-}" ]; then
        printf '%s\n' "$INSTALL_DIR" >>"$GITHUB_PATH"
    fi
    printf '%s\n' "$installed"
}

resolve_platform
main
