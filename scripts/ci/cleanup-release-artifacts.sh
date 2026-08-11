#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
POCKET_IC_STOPPER="$ROOT/scripts/ci/stop-owned-pocketic-servers.sh"
TEST_SCRATCH_PARENT="$ROOT/.tmp"
TEST_SCRATCH="${CANIC_TEST_SCRATCH:-}"
MAX_CARGO_CLEAN_ATTEMPTS=2
CLEANUP_MODE="${1:-all}"
status=0

cd "$ROOT" || exit 1

case "$CLEANUP_MODE" in
    all | --scratch-only) ;;
    *)
        echo "usage: $0 [--scratch-only]" >&2
        exit 2
        ;;
esac

clean_cargo_artifacts() {
    local attempt

    for ((attempt = 1; attempt <= MAX_CARGO_CLEAN_ATTEMPTS; attempt++)); do
        echo "==> clearing Cargo build artifacts (attempt $attempt/$MAX_CARGO_CLEAN_ATTEMPTS)"
        if cargo clean; then
            return 0
        fi

        if [[ "$attempt" -lt "$MAX_CARGO_CLEAN_ATTEMPTS" ]]; then
            echo "release cleanup will retry the transient Cargo clean failure" >&2
        fi
    done

    return 1
}

validate_test_scratch() {
    local parent name

    parent="${TEST_SCRATCH%/*}"
    name="${TEST_SCRATCH##*/}"
    if [[ "$parent" != "$TEST_SCRATCH_PARENT" ||
        ! "$name" =~ ^test-runtime\.[[:alnum:]]{6}$ ]]; then
        echo "release cleanup refuses unowned test scratch: $TEST_SCRATCH" >&2
        return 1
    fi
    if [[ -L "$TEST_SCRATCH_PARENT" || -L "$TEST_SCRATCH" ]]; then
        echo "release cleanup refuses symlinked test scratch: $TEST_SCRATCH" >&2
        return 1
    fi
    return 0
}

if [[ "$CLEANUP_MODE" == "all" ]]; then
    if ! clean_cargo_artifacts; then
        echo "release cleanup failed to clear Cargo build artifacts" >&2
        status=1
    fi
else
    echo "==> leaving Cargo build artifacts intact"
fi

if [[ -n "$TEST_SCRATCH" ]]; then
    if ! validate_test_scratch; then
        status=1
    elif [[ -e "$TEST_SCRATCH" ]]; then
        CANIC_TEST_SCRATCH="$TEST_SCRATCH" bash "$POCKET_IC_STOPPER"
        pocket_ic_cleanup_status=$?
        if [[ "$pocket_ic_cleanup_status" -ne 0 ]]; then
            echo "release cleanup retained scratch used by a live PocketIC server" >&2
            status=1
        else
            echo "==> clearing invocation-owned test scratch: ${TEST_SCRATCH##*/}"
        fi
        if [[ "$pocket_ic_cleanup_status" -eq 0 ]] && ! rm -rf -- "$TEST_SCRATCH"; then
            echo "release cleanup failed to clear invocation-owned test scratch" >&2
            status=1
        fi
    fi
elif [[ "$CLEANUP_MODE" == "--scratch-only" ]]; then
    echo "release cleanup has no invocation-owned test scratch to clear"
fi

if [[ -d "$TEST_SCRATCH_PARENT" && ! -L "$TEST_SCRATCH_PARENT" ]]; then
    if ! rmdir "$TEST_SCRATCH_PARENT" 2>/dev/null; then
        :
    fi
elif [[ -L "$TEST_SCRATCH_PARENT" ]]; then
    if [[ -n "$TEST_SCRATCH" ]]; then
        echo "release cleanup refuses symlinked repository scratch root: $TEST_SCRATCH_PARENT" >&2
        status=1
    fi
fi

exit "$status"
