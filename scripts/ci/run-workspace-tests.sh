#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INVENTORY="$ROOT/scripts/ci/workspace-test-inventory.tsv"
MODE="${1:-full}"
TARGETED_POCKETIC_TEST="${2:-}"
SUMMARY_LABELS=()
SUMMARY_DURATIONS=()
SUMMARY_KINDS=()
SUMMARY_RESULTS=()
FAILED_LABELS=()
HEAVY_BUILD_TARGETS_USED=0
PLAN_ONLY="${CANIC_TEST_PLAN_ONLY:-0}"
STEP_SUMMARY_INITIALIZED=0
POCKET_IC_SERVER_TTL_SECONDS=7200
POCKET_IC_SERVER_PID=""

case "$MODE" in
    fast | full | ordinary | pocketic) ;;
    targeted-pocketic)
        if [[ -z "$TARGETED_POCKETIC_TEST" ]]; then
            echo "targeted-pocketic requires one exact governed lib test" >&2
            exit 2
        fi
        ;;
    *)
        echo "usage: $0 <fast|full|ordinary|pocketic|targeted-pocketic> [exact-test]" >&2
        exit 2
        ;;
esac

cd "$ROOT"

case "$PLAN_ONLY" in
    0 | 1) ;;
    *)
        echo "CANIC_TEST_PLAN_ONLY must be 0 or 1" >&2
        exit 2
        ;;
esac

elapsed_seconds() {
    local started_at="$1"
    echo "$((SECONDS - started_at))s"
}

report_owned_pocketic_server_output() {
    if [[ -z "${CANIC_TEST_SCRATCH:-}" ]]; then
        return
    fi
    echo "==> shared PocketIC server stderr (last 40 lines)" >&2
    tail -40 "$CANIC_TEST_SCRATCH/pocketic.stderr" >&2 || true
    echo "==> shared PocketIC server stdout (last 40 lines)" >&2
    tail -40 "$CANIC_TEST_SCRATCH/pocketic.stdout" >&2 || true
}

report_owned_pocketic_server_resources() {
    local label="$1"
    local server_pid="$POCKET_IC_SERVER_PID"
    if [[ -z "$server_pid" || ! -r "/proc/$server_pid/status" ]]; then
        echo "==> shared PocketIC resources after $label: unavailable" >&2
        return
    fi

    local key value rss="unknown" high_water="unknown" threads="unknown"
    while IFS=: read -r key value; do
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"
        case "$key" in
            Threads) threads="$value" ;;
            VmHWM) high_water="$value" ;;
            VmRSS) rss="$value" ;;
        esac
    done <"/proc/$server_pid/status"
    echo "==> shared PocketIC resources after $label: rss=$rss high_water=$high_water threads=$threads"
}

stop_owned_pocketic_server() {
    local server_pid="$POCKET_IC_SERVER_PID"
    if [[ -z "$server_pid" ]]; then
        return
    fi
    POCKET_IC_SERVER_PID=""
    if kill -0 "$server_pid" 2>/dev/null; then
        kill -KILL "$server_pid" 2>/dev/null || true
    fi
    wait "$server_pid" 2>/dev/null || true
}

start_owned_pocketic_server() {
    if [[ "$PLAN_ONLY" -eq 1 ]]; then
        return
    fi
    if [[ -z "${CANIC_TEST_SCRATCH:-}" || ! -d "$CANIC_TEST_SCRATCH" ]]; then
        echo "PocketIC startup requires the governed private test scratch" >&2
        return 1
    fi

    local port_file="$CANIC_TEST_SCRATCH/pocket_ic_${BASHPID}.port"
    local stdout_file="$CANIC_TEST_SCRATCH/pocketic.stdout"
    local stderr_file="$CANIC_TEST_SCRATCH/pocketic.stderr"
    if [[ -e "$port_file" || -e "$stdout_file" || -e "$stderr_file" ]]; then
        echo "PocketIC startup files already exist in the governed scratch" >&2
        return 1
    fi

    "$POCKET_IC_BIN" \
        --ttl "$POCKET_IC_SERVER_TTL_SECONDS" \
        --hard-ttl "$POCKET_IC_SERVER_TTL_SECONDS" \
        --port-file "$port_file" \
        >"$stdout_file" 2>"$stderr_file" &
    POCKET_IC_SERVER_PID="$!"
    local attempt server_port
    for ((attempt = 0; attempt < 150; attempt++)); do
        if ! kill -0 "$POCKET_IC_SERVER_PID" 2>/dev/null; then
            local server_status=0
            wait "$POCKET_IC_SERVER_PID" || server_status="$?"
            POCKET_IC_SERVER_PID=""
            echo "PocketIC server exited before readiness (status $server_status)" >&2
            report_owned_pocketic_server_output
            return 1
        fi
        server_port=""
        if [[ -f "$port_file" ]]; then
            IFS= read -r server_port <"$port_file" || true
        fi
        if [[ "$server_port" =~ ^[0-9]+$ ]] &&
            ((server_port >= 1 && server_port <= 65535)); then
            export CANIC_POCKET_IC_SERVER_URL="http://127.0.0.1:$server_port/"
            echo "==> shared PocketIC server ready: $CANIC_POCKET_IC_SERVER_URL"
            return
        fi
        sleep 0.2
    done

    echo "PocketIC server did not publish a valid port within 30 seconds" >&2
    stop_owned_pocketic_server
    report_owned_pocketic_server_output
    return 1
}

record_summary() {
    SUMMARY_LABELS+=("$1")
    SUMMARY_DURATIONS+=("$2")
    SUMMARY_KINDS+=("$3")
    SUMMARY_RESULTS+=("$4")
}

append_step_summary() {
    local execution="$1"
    local elapsed="$2"
    local label="$3"
    local result="$4"

    if [[ "$PLAN_ONLY" -eq 1 || -z "${GITHUB_STEP_SUMMARY:-}" ]]; then
        return
    fi
    if [[ "$STEP_SUMMARY_INITIALIZED" -eq 0 ]]; then
        {
            echo "### Workspace test timing"
            echo
            echo "| Kind | Elapsed | Result | Suite |"
            echo "| --- | ---: | --- | --- |"
        } >>"$GITHUB_STEP_SUMMARY"
        STEP_SUMMARY_INITIALIZED=1
    fi
    printf '| `%s` | %s | %s | %s |\n' \
        "$execution" "$elapsed" "$result" "$label" >>"$GITHUB_STEP_SUMMARY"
}

print_summary() {
    local count="${#SUMMARY_LABELS[@]}"
    if [[ "$count" -eq 0 ]]; then
        return
    fi

    echo
    echo "==> workspace timing summary"
    printf '%-16s %-8s %-6s %s\n' "kind" "elapsed" "result" "label"
    printf '%-16s %-8s %-6s %s\n' "----" "-------" "------" "-----"

    local i
    for ((i = 0; i < count; i++)); do
        printf '%-16s %-8s %-6s %s\n' \
            "${SUMMARY_KINDS[$i]}" \
            "${SUMMARY_DURATIONS[$i]}" \
            "${SUMMARY_RESULTS[$i]}" \
            "${SUMMARY_LABELS[$i]}"
    done
}

finish_test_run() {
    print_summary
    if [[ "$PLAN_ONLY" -eq 1 ]]; then
        echo "WORKSPACE TEST PLAN RESOLVED: all requested suites were classified."
        return
    fi
    if [[ ${#FAILED_LABELS[@]} -eq 0 ]]; then
        echo "WORKSPACE TESTS PASSED: all requested suites succeeded."
        return
    fi
    printf 'WORKSPACE TESTS FAILED: %s\n' "${FAILED_LABELS[*]}" >&2
    return 1
}

require_ordinary_success_before_pocketic() {
    if [[ "$MODE" != "full" || "$PLAN_ONLY" -eq 1 || ${#FAILED_LABELS[@]} -eq 0 ]]; then
        return
    fi
    echo "ORDINARY TEST BARRIER FAILED: skipping the serial PocketIC suites." >&2
    finish_test_run
}

run_test() {
    local execution="$1"
    local label="$2"
    shift 2
    local cargo_args=()
    local libtest_args=()
    local parsing_libtest=0
    local argument
    for argument in "$@"; do
        if [[ "$argument" = "--" && "$parsing_libtest" -eq 0 ]]; then
            parsing_libtest=1
            continue
        fi
        if [[ "$parsing_libtest" -eq 0 ]]; then
            cargo_args+=("$argument")
        else
            libtest_args+=("$argument")
        fi
    done
    echo "==> $label"
    if [ "$PLAN_ONLY" -eq 1 ]; then
        printf '==> plan: cargo test --locked --no-fail-fast'
        printf ' %q' "${cargo_args[@]}"
        if [ "$execution" = "pocketic-serial" ]; then
            printf ' -- --test-threads=1 --nocapture'
        else
            printf ' -- --nocapture'
        fi
        if [[ "${#libtest_args[@]}" -gt 0 ]]; then
            printf ' %q' "${libtest_args[@]}"
        fi
        printf '\n'
        record_summary "$label" "0s" "$execution" "PLAN"
        return
    fi
    local started_at="$SECONDS"
    local status=0
    case "$execution" in
        parallel)
            cargo test --locked --no-fail-fast "${cargo_args[@]}" -- --nocapture \
                "${libtest_args[@]}" || status=$?
            ;;
        pocketic-serial)
            cargo test --locked --no-fail-fast "${cargo_args[@]}" -- --test-threads=1 --nocapture \
                "${libtest_args[@]}" || status=$?
            ;;
        *)
            echo "unknown test execution class: $execution" >&2
            exit 2
            ;;
    esac
    local elapsed
    elapsed="$(elapsed_seconds "$started_at")"
    if [[ "$execution" = "pocketic-serial" ]]; then
        report_owned_pocketic_server_resources "$label"
    fi
    if [[ "$status" -eq 0 ]]; then
        record_summary "$label" "$elapsed" "$execution" "PASS"
        echo "==> $label done in $elapsed"
        append_step_summary "$execution" "$elapsed" "$label" "PASS"
        return
    fi
    record_summary "$label" "$elapsed" "$execution" "FAIL"
    FAILED_LABELS+=("$label")
    echo "==> $label failed in $elapsed (exit $status)" >&2
    if [[ "$execution" = "pocketic-serial" ]]; then
        report_owned_pocketic_server_output
    fi
    append_step_summary "$execution" "$elapsed" "$label" "FAIL ($status)"
    return 0
}

run_parallel_test() {
    local label="$1"
    shift
    run_test parallel "$label" "$@"
}

run_serial_pocketic_test() {
    local label="$1"
    shift
    run_test pocketic-serial "$label" "$@"
}

run_inventory_tests() {
    local label="$1"
    local package="$2"
    local execution="$3"
    local suite="$4"
    local row_package row_target release_lane row_execution row_suite
    local selected=0
    local cargo_args=(-p "$package")

    while IFS=$'\t' read -r row_package row_target release_lane row_execution row_suite; do
        [ "$row_package" = "$package" ] || continue
        [ "$row_execution" = "$execution" ] || continue
        [ "$row_suite" = "$suite" ] || continue
        if [ "$MODE" = "fast" ] && [ "$release_lane" != "fast" ]; then
            continue
        fi
        cargo_args+=(--test "$row_target")
        selected=$((selected + 1))
    done < <(tail -n +2 "$INVENTORY")

    [ "$selected" -gt 0 ] || {
        echo "no $MODE inventory targets selected for $package/$execution/$suite" >&2
        exit 2
    }
    run_test "$execution" "$label" "${cargo_args[@]}"
}

clear_pocketic_build_targets() {
    local label="$1"
    local ci_only="${2:-0}"
    local cleared=0
    local target_dir
    local target_dirs=(
        "target/icp-build"
        "target/canic-wasm"
        "target/pic-wasm"
        "target/pic-runtime-wasm"
        "target/pic-wasm-no-test-material"
        "target/fleet-coordinator"
        "target/fleet-registry-sync"
        "target/standalone-blob_storage_cashier_mock"
        "target/standalone-blob_storage_probe"
        "target/standalone-leaf_probe"
        "target/standalone-payload_limit_probe"
        "target/standalone-root-probe"
        "target/standalone-scaling_probe"
    )

    if [[ "$ci_only" -eq 1 ]]; then
        # CI clears between heavy suites so one runner never carries multiple
        # isolated Wasm targets at once.
        case "${CI:-}" in
            1 | true | TRUE | yes | YES) ;;
            *) return ;;
        esac
    fi

    # CI bounds transient Wasm disk use. Local runs retain exact build caches
    # until the maintainer explicitly runs `make clean-wasm`.
    for target_dir in "${target_dirs[@]}"; do
        if [[ ! -e "$target_dir" ]]; then
            continue
        fi

        if [[ "$cleared" -eq 0 ]]; then
            echo "==> clearing transient Wasm build targets: $label"
            cleared=1
        fi
        rm -rf "$target_dir" || echo "warning: failed to clear $target_dir" >&2
    done
}

cleanup_heavy_build_targets() {
    if [[ "$HEAVY_BUILD_TARGETS_USED" -eq 1 ]]; then
        clear_pocketic_build_targets "workspace test exit" 1
    fi
}

cleanup_workspace_test_run() {
    local run_status="$?"
    local cleanup_status=0

    trap - EXIT INT TERM
    stop_owned_pocketic_server || cleanup_status="$?"
    cleanup_heavy_build_targets || cleanup_status="$?"
    if [[ "$run_status" -ne 0 ]]; then
        exit "$run_status"
    fi
    exit "$cleanup_status"
}

run_pic_inventory_tests() {
    local label="$1"
    local suite="$2"
    if [[ "$PLAN_ONLY" -eq 0 && "$HEAVY_BUILD_TARGETS_USED" -eq 0 ]]; then
        HEAVY_BUILD_TARGETS_USED=1
        clear_pocketic_build_targets "before PocketIC integration suites" 1
    fi
    run_inventory_tests "$label" canic-tests pocketic-serial "$suite"
}

trap cleanup_workspace_test_run EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

bash scripts/ci/check-workspace-test-inventory.sh

if [ "$PLAN_ONLY" -eq 0 ]; then
    # Role-package contract tests inspect the Wasm graph with locked offline Cargo
    # metadata. Populate the complete locked graph once so results do not depend on
    # whether the restored Cargo cache contains every target and host/build package.
    if [[ ("$MODE" == "full" || "$MODE" == "pocketic" || "$MODE" == "targeted-pocketic") && -z "${POCKET_IC_BIN:-}" ]]; then
        POCKET_IC_BIN="$(bash scripts/ci/install-pocketic.sh)"
        export POCKET_IC_BIN
        echo "==> using pinned PocketIC server binary: $POCKET_IC_BIN"
    else
        bash scripts/ci/check-pocketic-version-alignment.sh
    fi
    echo "==> prefetching locked dependency graph for offline metadata checks"
    cargo fetch --locked
fi

# Run ordinary unit/lib/bin tests with libtest's default parallelism. The
# internal harness remains separate because its library contains PocketIC
# journeys protected by process-local fixture serialization.
if [[ "$MODE" != "pocketic" && "$MODE" != "targeted-pocketic" ]]; then
    run_parallel_test \
        "workspace parallel lib/bin tests" \
        --workspace \
        --lib \
        --bins \
        --exclude canic-testing-internal
    run_parallel_test \
        "canic-testing-internal fast lib tests" \
        -p canic-testing-internal \
        --lib \
        pic::governed_suite::governed_fast_internal_suite \
        -- \
        --exact \
        --ignored
fi

if [[ "$MODE" == "fast" ]]; then
    run_inventory_tests "fast release-surface integration tests" canic parallel ordinary
    finish_test_run
    exit 0
fi

if [[ "$MODE" != "pocketic" && "$MODE" != "targeted-pocketic" ]]; then
    # Every checked-in top-level integration target is classified by the
    # guarded inventory. Parallel-safe targets form an independently runnable
    # CI lane before the expensive PocketIC work.
    run_inventory_tests "canic-cli integration tests" canic-cli parallel ordinary
    run_inventory_tests "canic-core integration tests" canic-core parallel ordinary
    run_inventory_tests \
        "canic-testing-internal integration tests" \
        canic-testing-internal \
        parallel \
        ordinary
    run_inventory_tests "canic integration tests" canic parallel ordinary

    if [[ "$MODE" == "ordinary" ]]; then
        finish_test_run
        exit 0
    fi
fi

require_ordinary_success_before_pocketic
start_owned_pocketic_server

if [[ "$MODE" == "targeted-pocketic" ]]; then
    if [[ "$TARGETED_POCKETIC_TEST" = "pic::governed_suite::governed_serial_pocketic_suite" ]]; then
        run_serial_pocketic_test \
            "targeted governed canic-testing-internal PocketIC suite" \
            -p canic-testing-internal \
            --lib \
            "$TARGETED_POCKETIC_TEST" \
            -- \
            --exact \
            --ignored
    else
        run_serial_pocketic_test \
            "targeted canic-testing-internal PocketIC proof" \
            -p canic-testing-internal \
            --lib \
            "$TARGETED_POCKETIC_TEST" \
            -- \
            --exact
    fi
    finish_test_run
    exit 0
fi

# One governed harness calls every internal PocketIC scenario in explicit
# order, reports each result immediately and catches failures until the suite
# boundary. Keeping one Rust process preserves its process-local artifact and
# baseline pools. The deployment-restore and autonomous Root-removal proofs
# remain the first two cases.
run_serial_pocketic_test \
    "canic-testing-internal ordered PocketIC suite" \
    -p canic-testing-internal \
    --lib \
    pic::governed_suite::governed_serial_pocketic_suite \
    -- \
    --exact \
    --ignored

# PocketIC-backed integration suites.
# Receipt, timer and lifecycle use the same internal-test build environment and
# target directory, so clear once before the group and retain Cargo freshness
# across the remaining binaries.
run_pic_inventory_tests "canic-tests runtime PocketIC suite" runtime
run_pic_inventory_tests "canic-tests blob-storage PocketIC suite" blob-storage
run_pic_inventory_tests "canic-tests payload-limit PocketIC suite" payload-limits
run_pic_inventory_tests "canic-tests instruction-audit PocketIC suite" instruction-audit

finish_test_run
