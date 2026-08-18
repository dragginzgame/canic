#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INVENTORY="$ROOT/scripts/ci/workspace-test-inventory.tsv"
MODE="${1:-full}"
SUMMARY_LABELS=()
SUMMARY_DURATIONS=()
SUMMARY_KINDS=()
SUMMARY_RESULTS=()
FAILED_LABELS=()
HEAVY_BUILD_TARGETS_USED=0
PLAN_ONLY="${CANIC_TEST_PLAN_ONLY:-0}"
STEP_SUMMARY_INITIALIZED=0

case "$MODE" in
    fast | full | ordinary | pocketic) ;;
    *)
        echo "usage: $0 <fast|full|ordinary|pocketic>" >&2
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
    if [[ "$status" -eq 0 ]]; then
        record_summary "$label" "$elapsed" "$execution" "PASS"
        echo "==> $label done in $elapsed"
        append_step_summary "$execution" "$elapsed" "$label" "PASS"
        return
    fi
    record_summary "$label" "$elapsed" "$execution" "FAIL"
    FAILED_LABELS+=("$label")
    echo "==> $label failed in $elapsed (exit $status)" >&2
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

run_pic_inventory_tests() {
    local label="$1"
    local suite="$2"
    if [[ "$PLAN_ONLY" -eq 0 && "$HEAVY_BUILD_TARGETS_USED" -eq 0 ]]; then
        HEAVY_BUILD_TARGETS_USED=1
        clear_pocketic_build_targets "before PocketIC integration suites" 1
    fi
    run_inventory_tests "$label" canic-tests pocketic-serial "$suite"
}

trap cleanup_heavy_build_targets EXIT

bash scripts/ci/check-workspace-test-inventory.sh

if [ "$PLAN_ONLY" -eq 0 ]; then
    # Role-package contract tests inspect the Wasm graph with locked offline Cargo
    # metadata. Populate the complete locked graph once so results do not depend on
    # whether the restored Cargo cache contains every target and host/build package.
    if [[ ("$MODE" == "full" || "$MODE" == "pocketic") && -z "${POCKET_IC_BIN:-}" ]]; then
        POCKET_IC_BIN="$(bash scripts/ci/install-pocketic.sh)"
        export POCKET_IC_BIN
        echo "==> using persistent PocketIC server: $POCKET_IC_BIN"
    else
        bash scripts/ci/check-pocketic-version-alignment.sh
    fi
    echo "==> prefetching locked dependency graph for offline metadata checks"
    cargo fetch --locked
fi

# Run ordinary unit/lib/bin tests with libtest's default parallelism. The
# internal harness remains separate because its library contains PocketIC
# journeys protected by process-local fixture serialization.
if [[ "$MODE" != "pocketic" ]]; then
    run_parallel_test \
        "workspace parallel lib/bin tests" \
        --workspace \
        --lib \
        --bins \
        --exclude canic-testing-internal
fi

if [[ "$MODE" == "fast" ]]; then
    # The internal crate's PocketIC journeys run in the dedicated PocketIC
    # lane. Compile its complete test harness here and retain its pure
    # embedded-config unit proof.
    run_parallel_test \
        "canic-testing-internal embedded config" \
        -p canic-testing-internal \
        --lib \
        pic::lifecycle::tests::init_payload_component_spec_matches_embedded_canister_config
    run_inventory_tests "fast release-surface integration tests" canic parallel ordinary
    finish_test_run
    exit 0
fi

if [[ "$MODE" != "pocketic" ]]; then
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

# The internal library owns several PocketIC journeys in its adjacent unit
# tests. Run the destructive autonomous-removal proof first so a regression is
# reported without waiting for the rest of the mixed harness. The remaining
# serial lane skips that exact test rather than paying for the journey twice.
ROOT_REMOVAL_TEST="pic::fleet_registry::baseline::tests::published_draining_root_autonomously_reaches_external_deletion_readiness"
run_serial_pocketic_test \
    "autonomous Root-removal PocketIC proof" \
    -p canic-testing-internal \
    --lib \
    "$ROOT_REMOVAL_TEST" \
    -- \
    --exact
run_serial_pocketic_test \
    "canic-testing-internal lib tests" \
    -p canic-testing-internal \
    --lib \
    -- \
    --skip "$ROOT_REMOVAL_TEST"

# PocketIC-backed integration suites.
# Receipt, timer and lifecycle use the same internal-test build environment and
# target directory, so clear once before the group and retain Cargo freshness
# across the remaining binaries.
run_pic_inventory_tests "canic-tests runtime PocketIC suite" runtime
run_pic_inventory_tests "canic-tests blob-storage PocketIC suite" blob-storage
run_pic_inventory_tests "canic-tests payload-limit PocketIC suite" payload-limits
run_pic_inventory_tests "canic-tests instruction-audit PocketIC suite" instruction-audit

finish_test_run
