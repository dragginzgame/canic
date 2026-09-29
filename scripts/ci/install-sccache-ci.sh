#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/../../tool-versions.env"

# This installer serves the Ubuntu x64 CI jobs. Local setup uses Cargo.
[[ "$(uname -s):$(uname -m)" == Linux:x86_64 ]] || {
    echo 'CI sccache installer requires Linux x86_64' >&2
    exit 1
}
install_dir="${CANIC_SCCACHE_INSTALL_DIR:-$HOME/.local/bin}"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
package="sccache-v${CANIC_SCCACHE_VERSION}-x86_64-unknown-linux-musl"
curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
    "https://github.com/mozilla/sccache/releases/download/v${CANIC_SCCACHE_VERSION}/${package}.tar.gz" \
    -o "$scratch/archive.tar.gz"
bash "$SCRIPT_DIR/verify-file-checksum.sh" sha256 \
    "$CANIC_SCCACHE_SHA256_LINUX_X64" "$scratch/archive.tar.gz"
tar -xzf "$scratch/archive.tar.gz" -C "$scratch" "$package/sccache"
candidate="$scratch/$package/sccache"
chmod +x "$candidate"
[[ "$("$candidate" --version)" == "sccache $CANIC_SCCACHE_VERSION" ]]
mkdir -p "$install_dir"
mv "$candidate" "$install_dir/sccache"
if [[ -n "${GITHUB_PATH:-}" ]]; then
    printf '%s\n' "$install_dir" >> "$GITHUB_PATH"
fi
printf '%s\n' "$install_dir/sccache"
