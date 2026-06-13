#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
if [ -z "${CANIC_ACTIONLINT_VERSION:-${ACTIONLINT_VERSION:-}}" ] &&
    [ -f "$ROOT_DIR/tool-versions.env" ]; then
    # shellcheck source=tool-versions.env
    source "$ROOT_DIR/tool-versions.env"
fi

VERSION="${1:-${CANIC_ACTIONLINT_VERSION:-${ACTIONLINT_VERSION:-}}}"
INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"

if [ -z "$VERSION" ]; then
    echo "missing actionlint version; set CANIC_ACTIONLINT_VERSION or update tool-versions.env" >&2
    exit 1
fi

platform() {
    local os
    local arch

    case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="darwin" ;;
    *)
        echo "unsupported actionlint platform: $(uname -s)" >&2
        exit 1
        ;;
    esac

    case "$(uname -m)" in
    x86_64 | amd64) arch="amd64" ;;
    arm64 | aarch64) arch="arm64" ;;
    *)
        echo "unsupported actionlint architecture: $(uname -m)" >&2
        exit 1
        ;;
    esac

    printf '%s_%s\n' "$os" "$arch"
}

main() {
    local version_no_v="${VERSION#v}"
    local archive="actionlint_${version_no_v}_$(platform).tar.gz"
    local url="https://github.com/rhysd/actionlint/releases/download/v${version_no_v}/${archive}"
    local tmp_dir

    tmp_dir="$(mktemp -d)"
    mkdir -p "$INSTALL_DIR"
    curl -fsSL "$url" | tar -xz -C "$tmp_dir" actionlint
    mv "$tmp_dir/actionlint" "$INSTALL_DIR/actionlint"
    chmod +x "$INSTALL_DIR/actionlint"
    rm -rf "$tmp_dir"

    printf '%s/actionlint\n' "$INSTALL_DIR"
}

main "$@"
