#!/usr/bin/env bash
set -euo pipefail

version="${1:-${CANIC_ICP_CLI_VERSION:-0.3.0}}"
cargo_bin_dir="$HOME/.cargo/bin"
installer_url="https://github.com/dfinity/icp-cli/releases/download/v$version/icp-cli-installer.sh"

if [ -z "$version" ]; then
    echo "missing ICP CLI version" >&2
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required to install icp-cli" >&2
    exit 1
fi

mkdir -p "$cargo_bin_dir"
export PATH="$cargo_bin_dir:$PATH"

if [ -n "${GITHUB_PATH:-}" ]; then
    printf '%s\n' "$cargo_bin_dir" >>"$GITHUB_PATH"
fi

curl --proto '=https' --tlsv1.2 -LsSf "$installer_url" | sh

if ! command -v icp >/dev/null 2>&1; then
    echo "icp-cli installer completed, but icp is not on PATH" >&2
    echo "expected $cargo_bin_dir to contain icp" >&2
    exit 1
fi

echo "icp ready: $(icp --version 2>&1) ($(command -v icp))" >&2
