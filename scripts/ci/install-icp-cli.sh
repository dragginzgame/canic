#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
if [ -z "${CANIC_ICP_CLI_VERSION:-}" ] && [ -f "$ROOT_DIR/tool-versions.env" ]; then
    # shellcheck source=tool-versions.env
    source "$ROOT_DIR/tool-versions.env"
fi

version="${1:-${CANIC_ICP_CLI_VERSION:-}}"
cargo_bin_dir="${CARGO_HOME:-$HOME/.cargo}/bin"
installer_url="https://github.com/dfinity/icp-cli/releases/download/v$version/icp-cli-installer.sh"

icp_version_matches() {
    local output="$1"
    local required_version="$2"

    case "$output" in
        *" $required_version"|*" $required_version "*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

latest_icp_cli_version() {
    local latest_url=""
    local latest_tag=""

    latest_url="$(
        curl -fsSIL -o /dev/null -w '%{url_effective}' \
            https://github.com/dfinity/icp-cli/releases/latest 2>/dev/null
    )" || return 1
    latest_tag="${latest_url##*/}"
    latest_tag="${latest_tag#v}"
    if [[ "$latest_tag" =~ ^[0-9]+(\.[0-9]+){1,2}([-+][0-9A-Za-z.-]+)?$ ]]; then
        printf '%s\n' "$latest_tag"
        return 0
    fi
    return 1
}

warn_if_newer_icp_cli_exists() {
    local pinned_version="$1"
    local latest_version=""

    if [ "${CANIC_ICP_CLI_LATEST_CHECK:-1}" = "0" ]; then
        return 0
    fi
    latest_version="$(latest_icp_cli_version)" || return 0
    if [ "$latest_version" != "$pinned_version" ]; then
        echo "warning: GitHub latest dfinity/icp-cli release is v$latest_version; Canic remains pinned to v$pinned_version in tool-versions.env" >&2
    fi
}

if [ -z "$version" ]; then
    echo "missing ICP CLI version; set CANIC_ICP_CLI_VERSION or update tool-versions.env" >&2
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required to install icp-cli" >&2
    exit 1
fi

mkdir -p "$cargo_bin_dir"
export PATH="$cargo_bin_dir:$PATH"
hash -r 2>/dev/null || true

if [ -n "${GITHUB_PATH:-}" ]; then
    printf '%s\n' "$cargo_bin_dir" >>"$GITHUB_PATH"
fi

curl --proto '=https' --tlsv1.2 -LsSf "$installer_url" | sh
hash -r 2>/dev/null || true

if ! command -v icp >/dev/null 2>&1; then
    echo "icp-cli installer completed, but icp is not on PATH" >&2
    echo "expected $cargo_bin_dir to contain icp" >&2
    exit 1
fi

icp_path="$(command -v icp)"
if ! icp_version_output="$(icp --version 2>&1)"; then
    echo "icp-cli installer completed, but icp is not working" >&2
    echo "$icp_version_output" >&2
    echo "resolved path: $icp_path" >&2
    exit 1
fi

if ! icp_version_matches "$icp_version_output" "$version"; then
    echo "icp-cli installer completed, but icp is not the requested version" >&2
    echo "found: $icp_version_output ($icp_path)" >&2
    echo "required: icp $version" >&2
    echo "expected install directory: $cargo_bin_dir" >&2
    exit 1
fi

echo "icp ready: $icp_version_output ($icp_path)" >&2
warn_if_newer_icp_cli_exists "$version"
