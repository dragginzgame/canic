#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEST_SCRATCH_PARENT="$ROOT/.tmp"
TEST_SCRATCH="${CANIC_TEST_SCRATCH:-}"
MAX_EXIT_WAIT_STEPS=50

fail() {
    echo "PocketIC cleanup failed: $1" >&2
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

is_owned_port_file() {
    local path="$1"
    local parent="${path%/*}"
    local name="${path##*/}"

    [[ "$parent" == "$TEST_SCRATCH" && "$name" =~ ^pocket_ic_[0-9]+\.port$ ]]
}

pid_is_owned_server() {
    local pid="$1"
    local command_line="/proc/$pid/cmdline"
    local previous argument
    local -a arguments

    arguments=()
    mapfile -d '' -t arguments 2>/dev/null <"$command_line" || return 1
    [[ "${#arguments[@]}" -gt 0 ]] || return 1
    [[ "${arguments[0]##*/}" == "pocket-ic" ]] || return 1

    previous=""
    for argument in "${arguments[@]}"; do
        if [[ "$previous" == "--port-file" ]] && is_owned_port_file "$argument"; then
            return 0
        fi
        previous="$argument"
    done
    return 1
}

collect_owned_server_pids() {
    local command_line pid

    OWNED_SERVER_PIDS=()
    for command_line in /proc/[0-9]*/cmdline; do
        pid="${command_line#/proc/}"
        pid="${pid%/cmdline}"
        if pid_is_owned_server "$pid"; then
            OWNED_SERVER_PIDS+=("$pid")
        fi
    done
}

validate_test_scratch
collect_owned_server_pids

if [[ "${#OWNED_SERVER_PIDS[@]}" -eq 0 ]]; then
    exit 0
fi

echo "==> stopping ${#OWNED_SERVER_PIDS[@]} invocation-owned PocketIC server(s)"
for pid in "${OWNED_SERVER_PIDS[@]}"; do
    if pid_is_owned_server "$pid"; then
        kill -KILL "$pid" 2>/dev/null || :
    fi
done

for ((step = 0; step < MAX_EXIT_WAIT_STEPS; step++)); do
    alive=0
    for pid in "${OWNED_SERVER_PIDS[@]}"; do
        if [[ -d "/proc/$pid" ]]; then
            alive=1
            break
        fi
    done
    if [[ "$alive" -eq 0 ]]; then
        exit 0
    fi
    sleep 0.1
done

fail "an invocation-owned PocketIC server remained after forced termination"
