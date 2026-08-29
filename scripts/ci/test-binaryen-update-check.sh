#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECK="$ROOT/scripts/dev/check-binaryen-update.sh"
# shellcheck source=/dev/null
source "$ROOT/tool-versions.env"

FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-binaryen-update-check.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/bin"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'if [ "${FAKE_CURL_FAIL:-0}" = "1" ]; then exit 22; fi' \
    'printf "%s" "${FAKE_CURL_URL:?}"' >"$FIXTURE/bin/curl"
chmod +x "$FIXTURE/bin/curl"

run_check() {
    PATH="$FIXTURE/bin:$PATH" \
        FAKE_CURL_FAIL="${2:-0}" \
        FAKE_CURL_URL="$1" \
        bash "$CHECK" 2>&1
}

next_version="$((CANIC_BINARYEN_VERSION + 1))"
output="$(run_check "https://github.com/WebAssembly/binaryen/releases/tag/version_$next_version")"
[[ "$output" == *"Binaryen update available: pinned $CANIC_BINARYEN_VERSION, latest $next_version"* ]] || {
    echo "Binaryen update check did not report a newer release" >&2
    exit 1
}

output="$(run_check "https://github.com/WebAssembly/binaryen/releases/tag/version_$CANIC_BINARYEN_VERSION")"
[[ "$output" == "Binaryen pin is current: $CANIC_BINARYEN_VERSION." ]] || {
    echo "Binaryen update check did not report the current pin" >&2
    exit 1
}

previous_version="$((CANIC_BINARYEN_VERSION - 1))"
output="$(run_check "https://github.com/WebAssembly/binaryen/releases/tag/version_$previous_version")"
[[ "$output" == *"Binaryen pin $CANIC_BINARYEN_VERSION is newer than the latest published release $previous_version"* ]] || {
    echo "Binaryen update check did not report a newer local pin" >&2
    exit 1
}

output="$(run_check 'https://github.com/WebAssembly/binaryen/releases/tag/not-a-version')"
[[ "$output" == *"unrecognized release URL"* ]] || {
    echo "Binaryen update check accepted a malformed release tag" >&2
    exit 1
}

output="$(run_check 'unused' 1)"
[[ "$output" == "Binaryen latest-version check unavailable; continuing with pinned version $CANIC_BINARYEN_VERSION." ]] || {
    echo "Binaryen update check did not remain informational after a network failure" >&2
    exit 1
}

echo "Binaryen update check tests passed"
