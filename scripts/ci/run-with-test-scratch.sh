#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEST_SCRATCH_PARENT="$ROOT/.tmp"
TEST_SCRATCH="${CANIC_TEST_SCRATCH:-}"
OWNS_TEST_SCRATCH=0

fail() {
    echo "test scratch runner failed: $1" >&2
    exit 1
}

validate_test_scratch() {
    local parent name

    parent="${TEST_SCRATCH%/*}"
    name="${TEST_SCRATCH##*/}"
    if [[ "$parent" != "$TEST_SCRATCH_PARENT" ||
        ! "$name" =~ ^test-runtime\.[[:alnum:]]{6}$ ]]; then
        fail "scratch path must be one direct private child of $TEST_SCRATCH_PARENT"
    fi
    [[ ! -L "$TEST_SCRATCH_PARENT" ]] ||
        fail "repository scratch parent may not be a symlink"
    [[ -d "$TEST_SCRATCH" && ! -L "$TEST_SCRATCH" ]] ||
        fail "scratch path must be an existing regular directory"
}

if [[ "$#" -eq 0 ]]; then
    echo "usage: $0 <command> [args...]" >&2
    exit 2
fi

if [[ -n "$TEST_SCRATCH" ]]; then
    validate_test_scratch
else
    [[ ! -L "$TEST_SCRATCH_PARENT" ]] ||
        fail "repository scratch parent may not be a symlink"
    mkdir -p "$TEST_SCRATCH_PARENT" || fail "cannot create repository scratch parent"
    [[ ! -L "$TEST_SCRATCH_PARENT" ]] ||
        fail "repository scratch parent became a symlink"
    TEST_SCRATCH="$(mktemp -d "$TEST_SCRATCH_PARENT/test-runtime.XXXXXX")" ||
        fail "cannot allocate private test scratch"
    OWNS_TEST_SCRATCH=1
fi

# shellcheck disable=SC2329 # Invoked by the EXIT trap.
finish() {
    local command_status="$?"
    local cleanup_status=0

    trap - EXIT INT TERM
    if [[ "$OWNS_TEST_SCRATCH" -eq 1 ]]; then
        CANIC_TEST_SCRATCH="$TEST_SCRATCH" \
            bash "$ROOT/scripts/ci/cleanup-release-artifacts.sh" --scratch-only
        cleanup_status=$?
    fi

    if [[ "$command_status" -ne 0 ]]; then
        exit "$command_status"
    fi
    exit "$cleanup_status"
}

trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

export CANIC_TEST_SCRATCH="$TEST_SCRATCH"
export TMPDIR="$TEST_SCRATCH"

"$@"
