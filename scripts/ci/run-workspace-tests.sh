#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-full}"
HARNESS_ARGS=(-- --test-threads=1 --nocapture)
SUMMARY_LABELS=()
SUMMARY_DURATIONS=()
SUMMARY_KINDS=()

elapsed_seconds() {
    local started_at="$1"
    echo "$((SECONDS - started_at))s"
}

record_summary() {
    SUMMARY_LABELS+=("$1")
    SUMMARY_DURATIONS+=("$2")
    SUMMARY_KINDS+=("$3")
}

print_summary() {
    local count="${#SUMMARY_LABELS[@]}"
    if [[ "$count" -eq 0 ]]; then
        return
    fi

    echo
    echo "==> workspace timing summary"
    printf '%-12s %-8s %s\n' "kind" "elapsed" "label"
    printf '%-12s %-8s %s\n' "----" "-------" "-----"

    local i
    for ((i = 0; i < count; i++)); do
        printf '%-12s %-8s %s\n' \
            "${SUMMARY_KINDS[$i]}" \
            "${SUMMARY_DURATIONS[$i]}" \
            "${SUMMARY_LABELS[$i]}"
    done
}

run_test() {
    local label="$1"
    shift
    echo "==> $label"
    local started_at="$SECONDS"
    cargo test "$@" "${HARNESS_ARGS[@]}"
    local elapsed
    elapsed="$(elapsed_seconds "$started_at")"
    echo "==> $label done in $elapsed"
    record_summary "$label" "$elapsed" "test"
}

clear_pocketic_wasm_targets() {
    local label="$1"
    local cleared=0
    local target_dir
    local target_dirs=(
        "target/pic-wasm"
        "target/pic-wasm-no-test-material"
        "target/delegation_root_stub_bootstrap_wasm_store"
        "target/delegation_root_stub_embedded_wasm"
    )

    if ! should_clear_pocketic_wasm_targets; then
        return
    fi

    for target_dir in "${target_dirs[@]}"; do
        if [[ ! -e "$target_dir" ]]; then
            continue
        fi

        if [[ "$cleared" -eq 0 ]]; then
            echo "==> clearing PocketIC wasm build targets before $label"
            cleared=1
        fi
        rm -rf "$target_dir"
    done
}

should_clear_pocketic_wasm_targets() {
    # CI keeps the aggressive cleanup to avoid runner disk exhaustion; local
    # runs keep Cargo's wasm build cache unless cleanup is explicitly requested.
    case "${CANIC_CLEAR_PIC_WASM_TARGETS:-}" in
        1 | true | TRUE | yes | YES)
            return 0
            ;;
        0 | false | FALSE | no | NO)
            return 1
            ;;
    esac

    case "${CI:-}" in
        1 | true | TRUE | yes | YES)
            return 0
            ;;
    esac

    return 1
}

run_pic_test() {
    local label="$1"
    shift
    clear_pocketic_wasm_targets "$label"
    run_test "$label" "$@"
}

# Compile and run all unit/lib/bin tests together first.
run_test "workspace lib/bin tests" --workspace --lib --bins
run_test "canic icp-refill doc tests" -p canic --features icp-refill --doc

# Keep cheap release-surface contract tests in both the full and fast lanes so
# version bumps and tagged installer drift fail before PocketIC-heavy work.
run_test "canic protocol_surface" -p canic --test protocol_surface
run_test "canic install_script_surface" -p canic --test install_script_surface
run_test "canic reference_surface" -p canic --test reference_surface

if [[ "$MODE" == "fast" ]]; then
    print_summary
    exit 0
fi

# Keep non-PocketIC integration tests explicit so the heavy PocketIC suites can
# run in a deterministic order without sitting behind a shared runtime lock.
run_test "canic control_plane_facade" -p canic --test control_plane_facade
run_test "canic workspace_manifest" -p canic --test workspace_manifest
run_test "canic-core trap_guard" -p canic-core --test trap_guard

# PocketIC-backed integration suites.
run_pic_test "canic-tests pic_intent_race" -p canic-tests --test pic_intent_race
run_pic_test "canic-tests pic_sharding_bootstrap" -p canic-tests --test pic_sharding_bootstrap
run_pic_test "canic-tests pic_role_attestation" -p canic-tests --test pic_role_attestation
run_pic_test "canic-tests lifecycle_boundary" -p canic-tests --test lifecycle_boundary
run_pic_test "canic-tests root_suite" -p canic-tests --test root_suite
run_pic_test "canic-tests root_wasm_store_reconcile" -p canic-tests --test root_wasm_store_reconcile
run_pic_test "canic-tests instruction_audit" -p canic-tests --test instruction_audit

print_summary
