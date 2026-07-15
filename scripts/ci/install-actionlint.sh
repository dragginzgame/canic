#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

VERSION="$CANIC_ACTIONLINT_VERSION"
INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=""

if [ "$#" -ne 0 ]; then
    echo "usage: install-actionlint.sh" >&2
    exit 1
fi

resolve_platform() {
    case "$(uname -s):$(uname -m)" in
    Darwin:x86_64 | Darwin:amd64)
        platform="darwin_amd64"
        checksum="$CANIC_ACTIONLINT_SHA256_DARWIN_AMD64"
        ;;
    Darwin:arm64 | Darwin:aarch64)
        platform="darwin_arm64"
        checksum="$CANIC_ACTIONLINT_SHA256_DARWIN_ARM64"
        ;;
    Linux:x86_64 | Linux:amd64)
        platform="linux_amd64"
        checksum="$CANIC_ACTIONLINT_SHA256_LINUX_AMD64"
        ;;
    Linux:arm64 | Linux:aarch64)
        platform="linux_arm64"
        checksum="$CANIC_ACTIONLINT_SHA256_LINUX_ARM64"
        ;;
    *)
        echo "unsupported actionlint platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local version_no_v="${VERSION#v}"
    local archive="actionlint_${version_no_v}_${platform}.tar.gz"
    local url="https://github.com/rhysd/actionlint/releases/download/v${version_no_v}/${archive}"
    local installed
    local candidate
    local version_output

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    mkdir -p "$INSTALL_DIR"
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$checksum" "$TMP_DIR/$archive"
    tar -xzf "$TMP_DIR/$archive" -C "$TMP_DIR" actionlint
    candidate="$TMP_DIR/actionlint"
    chmod +x "$candidate"
    version_output="$("$candidate" -version 2>&1)"
    case "$version_output" in
    *"$VERSION"*) ;;
    *)
        echo "installed actionlint does not report the pinned version" >&2
        echo "expected: $VERSION" >&2
        echo "actual:   $version_output" >&2
        exit 1
        ;;
    esac

    installed="$INSTALL_DIR/actionlint"
    mv "$candidate" "$installed"
    printf '%s\n' "$installed"
}

resolve_platform
main
