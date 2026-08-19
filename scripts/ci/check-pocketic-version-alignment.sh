#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOCK="$ROOT/Cargo.lock"
TOOLS="$ROOT/tool-versions.env"

if [ "$#" -ne 0 ]; then
    echo "usage: check-pocketic-version-alignment.sh" >&2
    exit 1
fi

# shellcheck source=/dev/null
source "$TOOLS"

mapfile -t locked_versions < <(
    awk '
        $0 == "[[package]]" {
            package = ""
            next
        }
        $0 == "name = \"pocket-ic\"" {
            package = "pocket-ic"
            next
        }
        package == "pocket-ic" && /^version = "/ {
            version = $0
            sub(/^version = "/, "", version)
            sub(/"$/, "", version)
            print version
        }
    ' "$LOCK"
)

if [ "${#locked_versions[@]}" -ne 1 ]; then
    echo "PocketIC version alignment failed: Cargo.lock must resolve exactly one pocket-ic package" >&2
    exit 1
fi

locked_version="${locked_versions[0]}"
if [ "$locked_version" != "$CANIC_POCKET_IC_VERSION" ]; then
    echo "PocketIC version alignment failed: Cargo.lock resolves $locked_version but tool-versions.env pins $CANIC_POCKET_IC_VERSION" >&2
    exit 1
fi

if [ -n "${POCKET_IC_BIN:-}" ]; then
    if [ ! -f "$POCKET_IC_BIN" ] || [ ! -x "$POCKET_IC_BIN" ]; then
        echo "PocketIC version alignment failed: POCKET_IC_BIN must be an executable file: $POCKET_IC_BIN" >&2
        exit 1
    fi
    bash "$ROOT/scripts/ci/verify-file-checksum.sh" \
        sha256 "$CANIC_POCKET_IC_BINARY_SHA256_LINUX_X86_64" "$POCKET_IC_BIN"
fi

echo "PocketIC version alignment passed ($locked_version)"
