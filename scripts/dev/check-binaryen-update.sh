#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT/tool-versions.env"

LATEST_RELEASE_URL="https://github.com/WebAssembly/binaryen/releases/latest"

latest_url=""
if ! latest_url="$({
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 \
        --location --silent --show-error --fail \
        --output /dev/null --write-out '%{url_effective}' \
        "$LATEST_RELEASE_URL"
} 2>/dev/null)"; then
    printf 'Binaryen latest-version check unavailable; continuing with pinned version %s.\n' \
        "$CANIC_BINARYEN_VERSION" >&2
    exit 0
fi

latest_url="${latest_url%/}"
latest_tag="${latest_url##*/}"
if [[ ! "$latest_tag" =~ ^version_([1-9][0-9]*)$ ]]; then
    printf 'Binaryen latest-version check returned an unrecognized release URL: %s\n' \
        "$latest_url" >&2
    exit 0
fi
latest_version="${BASH_REMATCH[1]}"

if [ "$latest_version" -gt "$CANIC_BINARYEN_VERSION" ]; then
    printf 'Binaryen update available: pinned %s, latest %s (%s).\n' \
        "$CANIC_BINARYEN_VERSION" "$latest_version" "$latest_url" >&2
elif [ "$latest_version" -eq "$CANIC_BINARYEN_VERSION" ]; then
    printf 'Binaryen pin is current: %s.\n' "$CANIC_BINARYEN_VERSION" >&2
else
    printf 'Binaryen pin %s is newer than the latest published release %s (%s).\n' \
        "$CANIC_BINARYEN_VERSION" "$latest_version" "$latest_url" >&2
fi
