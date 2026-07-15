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
        package_platform="darwin-arm64"
        checksum="$CANIC_IC_WASM_SHA512_DARWIN_ARM64"
        ;;
    Darwin:x86_64 | Darwin:amd64)
        package_platform="darwin-x64"
        checksum="$CANIC_IC_WASM_SHA512_DARWIN_X64"
        ;;
    Linux:arm64 | Linux:aarch64)
        package_platform="linux-arm64"
        checksum="$CANIC_IC_WASM_SHA512_LINUX_ARM64"
        ;;
    Linux:x86_64 | Linux:amd64)
        package_platform="linux-x64"
        checksum="$CANIC_IC_WASM_SHA512_LINUX_X64"
        ;;
    *)
        echo "unsupported ic-wasm platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local package="ic-wasm-${package_platform}"
    local archive="${package}-${CANIC_IC_WASM_VERSION}.tgz"
    local url="https://registry.npmjs.org/@icp-sdk/${package}/-/${archive}"
    local installed
    local candidate
    local version_output

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha512 "$checksum" "$TMP_DIR/$archive"
    tar -xzf "$TMP_DIR/$archive" -C "$TMP_DIR" package/bin/ic-wasm

    candidate="$TMP_DIR/package/bin/ic-wasm"
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
