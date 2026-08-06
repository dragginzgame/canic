#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEST_SCRATCH_PARENT="$ROOT/.tmp"
TEST_SCRATCH="$TEST_SCRATCH_PARENT/test-runtime"
status=0

cd "$ROOT" || exit 1

echo "==> clearing Cargo build artifacts"
if ! cargo clean; then
    echo "release cleanup failed to clear Cargo build artifacts" >&2
    status=1
fi

if [[ -L "$TEST_SCRATCH_PARENT" ]]; then
    echo "release cleanup refuses symlinked repository scratch root: $TEST_SCRATCH_PARENT" >&2
    status=1
elif [[ -e "$TEST_SCRATCH" || -L "$TEST_SCRATCH" ]]; then
    echo "==> clearing repository-owned test scratch: .tmp/test-runtime"
    if ! rm -rf -- "$TEST_SCRATCH"; then
        echo "release cleanup failed to clear repository-owned test scratch" >&2
        status=1
    fi
fi

if [[ -d "$TEST_SCRATCH_PARENT" ]]; then
    rmdir "$TEST_SCRATCH_PARENT" 2>/dev/null || true
fi

exit "$status"
