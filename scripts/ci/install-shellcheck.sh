#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

VERSION="$CANIC_SHELLCHECK_VERSION"
INSTALL_DIR="${SHELLCHECK_INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=""

if [ "$#" -ne 0 ]; then
    echo "usage: install-shellcheck.sh" >&2
    exit 1
fi

resolve_platform() {
    case "$(uname -s):$(uname -m)" in
    Darwin:arm64 | Darwin:aarch64)
        platform="darwin.aarch64"
        checksum="$CANIC_SHELLCHECK_SHA256_DARWIN_AARCH64"
        ;;
    Darwin:x86_64 | Darwin:amd64)
        platform="darwin.x86_64"
        checksum="$CANIC_SHELLCHECK_SHA256_DARWIN_X86_64"
        ;;
    Linux:arm64 | Linux:aarch64)
        platform="linux.aarch64"
        checksum="$CANIC_SHELLCHECK_SHA256_LINUX_AARCH64"
        ;;
    Linux:x86_64 | Linux:amd64)
        platform="linux.x86_64"
        checksum="$CANIC_SHELLCHECK_SHA256_LINUX_X86_64"
        ;;
    *)
        echo "unsupported ShellCheck platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local version_no_v="${VERSION#v}"
    local release_dir="shellcheck-v${version_no_v}"
    local archive
    local url
    local installed
    local candidate
    local version_output

    archive="${release_dir}.${platform}.tar.xz"
    url="https://github.com/koalaman/shellcheck/releases/download/v${version_no_v}/${archive}"

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    mkdir -p "$INSTALL_DIR"
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$checksum" "$TMP_DIR/$archive"
    tar -xJf "$TMP_DIR/$archive" -C "$TMP_DIR"
    candidate="$TMP_DIR/$release_dir/shellcheck"
    chmod +x "$candidate"
    version_output="$("$candidate" --version 2>&1)"
    case "$version_output" in
    *"version: $VERSION"*) ;;
    *)
        echo "installed ShellCheck does not report the pinned version" >&2
        echo "expected: $VERSION" >&2
        echo "actual:   $version_output" >&2
        exit 1
        ;;
    esac

    installed="$INSTALL_DIR/shellcheck"
    mv "$candidate" "$installed"
    printf '%s\n' "$installed"
}

resolve_platform
main
