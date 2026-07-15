#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

INSTALL_DIR="${ICP_CLI_INSTALL_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}"
TMP_DIR=""

if [ "$#" -ne 0 ]; then
    echo "usage: install-icp-cli.sh" >&2
    exit 1
fi

resolve_platform() {
    case "$(uname -s):$(uname -m)" in
    Darwin:arm64 | Darwin:aarch64)
        target="aarch64-apple-darwin"
        checksum="$CANIC_ICP_CLI_SHA256_AARCH64_APPLE_DARWIN"
        ;;
    Darwin:x86_64 | Darwin:amd64)
        target="x86_64-apple-darwin"
        checksum="$CANIC_ICP_CLI_SHA256_X86_64_APPLE_DARWIN"
        ;;
    Linux:arm64 | Linux:aarch64)
        target="aarch64-unknown-linux-gnu"
        checksum="$CANIC_ICP_CLI_SHA256_AARCH64_UNKNOWN_LINUX_GNU"
        ;;
    Linux:x86_64 | Linux:amd64)
        target="x86_64-unknown-linux-gnu"
        checksum="$CANIC_ICP_CLI_SHA256_X86_64_UNKNOWN_LINUX_GNU"
        ;;
    *)
        echo "unsupported ICP CLI platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local release_dir="icp-cli-$target"
    local archive="${release_dir}.tar.xz"
    local url="https://github.com/dfinity/icp-cli/releases/download/v${CANIC_ICP_CLI_VERSION}/${archive}"
    local installed
    local candidate
    local version_output

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$checksum" "$TMP_DIR/$archive"
    tar -xJf "$TMP_DIR/$archive" -C "$TMP_DIR" "$release_dir/icp"

    candidate="$TMP_DIR/$release_dir/icp"
    chmod +x "$candidate"
    version_output="$("$candidate" --version 2>&1)"
    case "$version_output" in
    *" $CANIC_ICP_CLI_VERSION" | *" $CANIC_ICP_CLI_VERSION "*) ;;
    *)
        echo "installed ICP CLI does not report the pinned version" >&2
        echo "expected: $CANIC_ICP_CLI_VERSION" >&2
        echo "actual:   $version_output" >&2
        exit 1
        ;;
    esac

    mkdir -p "$INSTALL_DIR"
    installed="$INSTALL_DIR/icp"
    mv "$candidate" "$installed"
    if [ -n "${GITHUB_PATH:-}" ]; then
        printf '%s\n' "$INSTALL_DIR" >>"$GITHUB_PATH"
    fi
    printf '%s\n' "$installed"
}

resolve_platform
main
