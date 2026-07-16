#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

VERSION="$CANIC_GITLEAKS_VERSION"
INSTALL_DIR="${GITLEAKS_INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=""

if [ "$#" -ne 0 ]; then
    echo "usage: install-gitleaks.sh" >&2
    exit 1
fi

resolve_platform() {
    case "$(uname -s):$(uname -m)" in
    Darwin:x86_64 | Darwin:amd64)
        platform="darwin_x64"
        checksum="$CANIC_GITLEAKS_SHA256_DARWIN_X64"
        ;;
    Darwin:arm64 | Darwin:aarch64)
        platform="darwin_arm64"
        checksum="$CANIC_GITLEAKS_SHA256_DARWIN_ARM64"
        ;;
    Linux:x86_64 | Linux:amd64)
        platform="linux_x64"
        checksum="$CANIC_GITLEAKS_SHA256_LINUX_X64"
        ;;
    Linux:arm64 | Linux:aarch64)
        platform="linux_arm64"
        checksum="$CANIC_GITLEAKS_SHA256_LINUX_ARM64"
        ;;
    *)
        echo "unsupported gitleaks platform: $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
    esac
}

main() {
    local version_no_v="${VERSION#v}"
    local archive="gitleaks_${version_no_v}_${platform}.tar.gz"
    local url="https://github.com/gitleaks/gitleaks/releases/download/v${version_no_v}/${archive}"
    local candidate
    local installed
    local version_output

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    mkdir -p "$INSTALL_DIR"
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -o "$TMP_DIR/$archive" "$url"
    bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 "$checksum" "$TMP_DIR/$archive"
    tar -xzf "$TMP_DIR/$archive" -C "$TMP_DIR" gitleaks
    candidate="$TMP_DIR/gitleaks"
    chmod +x "$candidate"
    version_output="$("$candidate" version 2>&1)"
    if [ "$version_output" != "$VERSION" ]; then
        echo "installed gitleaks does not report the pinned version" >&2
        echo "expected: $VERSION" >&2
        echo "actual:   $version_output" >&2
        exit 1
    fi

    installed="$INSTALL_DIR/gitleaks"
    mv "$candidate" "$installed"
    printf '%s\n' "$installed"
}

resolve_platform
main
