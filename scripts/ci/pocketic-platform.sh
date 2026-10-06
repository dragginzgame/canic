#!/usr/bin/env bash
# Print the pinned archive, archive digest and native executable digest.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"
# shellcheck source=/dev/null
source "$ROOT_DIR/scripts/ci/ic-tool-pins.sh"

if [ "$#" -ne 0 ]; then
    echo "usage: pocketic-platform.sh" >&2
    exit 1
fi

case "$(uname -s):$(uname -m)" in
Darwin:arm64 | Darwin:aarch64)
    printf '%s\t%s\t%s\n' pocket-ic-arm64-darwin.gz \
        "$(canic_ic_tool_pin pocket-ic darwin-arm64 4)" "$CANIC_POCKET_IC_BINARY_SHA256_DARWIN_ARM64"
    ;;
Darwin:x86_64 | Darwin:amd64)
    printf '%s\t%s\t%s\n' pocket-ic-x86_64-darwin.gz \
        "$(canic_ic_tool_pin pocket-ic darwin-x86_64 4)" "$CANIC_POCKET_IC_BINARY_SHA256_DARWIN_X86_64"
    ;;
Linux:x86_64 | Linux:amd64)
    printf '%s\t%s\t%s\n' pocket-ic-x86_64-linux.gz \
        "$(canic_ic_tool_pin pocket-ic linux-x86_64 4)" "$CANIC_POCKET_IC_BINARY_SHA256_LINUX_X86_64"
    ;;
*)
    echo "unsupported PocketIC platform: $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac
