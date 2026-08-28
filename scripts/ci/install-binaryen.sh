#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

INSTALL_DIR="${BINARYEN_INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=""

resolve_platform() {
    case "$(uname -s):$(uname -m)" in
    Darwin:arm64 | Darwin:aarch64)
        archive_platform="arm64-macos"
        checksum="$CANIC_BINARYEN_SHA256_DARWIN_ARM64"
        executable_checksum="$CANIC_BINARYEN_WASM_OPT_SHA256_DARWIN_ARM64"
        ;;
    Darwin:x86_64 | Darwin:amd64)
        archive_platform="x86_64-macos"
        checksum="$CANIC_BINARYEN_SHA256_DARWIN_X64"
        executable_checksum="$CANIC_BINARYEN_WASM_OPT_SHA256_DARWIN_X64"
        ;;
    Linux:x86_64 | Linux:amd64)
        archive_platform="x86_64-linux"
        checksum="$CANIC_BINARYEN_SHA256_LINUX_X64"
        executable_checksum="$CANIC_BINARYEN_WASM_OPT_SHA256_LINUX_X64"
        ;;
    *)
        echo "unsupported Binaryen platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local package="binaryen-version_${CANIC_BINARYEN_VERSION}"
    local archive="${package}-${archive_platform}.tar.gz"
    local url="https://github.com/WebAssembly/binaryen/releases/download/version_${CANIC_BINARYEN_VERSION}/${archive}"
    local candidate
    local installed
    local version_output

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$checksum" "$TMP_DIR/$archive"
    tar -xzf "$TMP_DIR/$archive" -C "$TMP_DIR" "$package/bin/wasm-opt"

    candidate="$TMP_DIR/$package/bin/wasm-opt"
    chmod +x "$candidate"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$executable_checksum" "$candidate"
    version_output="$("$candidate" --version 2>&1)"
    if [ "$version_output" != "wasm-opt version $CANIC_BINARYEN_VERSION (version_$CANIC_BINARYEN_VERSION)" ]; then
        echo "installed Binaryen does not report the pinned version" >&2
        echo "expected: wasm-opt version $CANIC_BINARYEN_VERSION (version_$CANIC_BINARYEN_VERSION)" >&2
        echo "actual:   $version_output" >&2
        exit 1
    fi

    mkdir -p "$INSTALL_DIR"
    installed="$INSTALL_DIR/wasm-opt"
    mv "$candidate" "$installed"
    if [ -n "${GITHUB_PATH:-}" ]; then
        printf '%s\n' "$INSTALL_DIR" >>"$GITHUB_PATH"
    fi
    printf '%s\n' "$installed"
}

resolve_platform
main
