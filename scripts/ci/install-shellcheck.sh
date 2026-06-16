#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [ -z "${CANIC_SHELLCHECK_VERSION:-}" ] && [ -f "$ROOT_DIR/tool-versions.env" ]; then
    # shellcheck source=tool-versions.env
    source "$ROOT_DIR/tool-versions.env"
fi

VERSION="${1:-${CANIC_SHELLCHECK_VERSION:-}}"
INSTALL_DIR="${SHELLCHECK_INSTALL_DIR:-$HOME/.local/bin}"

if [ -z "$VERSION" ]; then
    echo "missing ShellCheck version; set CANIC_SHELLCHECK_VERSION or update tool-versions.env" >&2
    exit 1
fi

platform() {
    local os
    local arch

    case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="darwin" ;;
    *)
        echo "unsupported ShellCheck platform: $(uname -s)" >&2
        exit 1
        ;;
    esac

    case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *)
        echo "unsupported ShellCheck architecture: $(uname -m)" >&2
        exit 1
        ;;
    esac

    printf '%s.%s\n' "$os" "$arch"
}

main() {
    local version_no_v="${VERSION#v}"
    local release_dir="shellcheck-v${version_no_v}"
    local archive
    local url
    local tmp_dir

    archive="${release_dir}.$(platform).tar.gz"
    url="https://github.com/koalaman/shellcheck/releases/download/v${version_no_v}/${archive}"

    tmp_dir="$(mktemp -d)"
    mkdir -p "$INSTALL_DIR"
    curl -fsSL "$url" | tar -xz -C "$tmp_dir"
    mv "$tmp_dir/$release_dir/shellcheck" "$INSTALL_DIR/shellcheck"
    chmod +x "$INSTALL_DIR/shellcheck"
    rm -rf "$tmp_dir"

    printf '%s/shellcheck\n' "$INSTALL_DIR"
}

main "$@"
