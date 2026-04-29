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

prebuild_root_test_artifacts() {
    local label="prebuild local DFX artifacts for PocketIC root suites"
    echo "==> $label"
    local started_at="$SECONDS"
    bash scripts/ci/build-ci-wasm-artifacts.sh
    local elapsed
    elapsed="$(elapsed_seconds "$started_at")"
    echo "==> $label done in $elapsed"
    record_summary "$label" "$elapsed" "prebuild"
}

# Compile and run all unit/lib/bin tests together first.
run_test "workspace lib/bin tests" --workspace --lib --bins

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
prebuild_root_test_artifacts
run_test "canic-tests pic_intent_race" -p canic-tests --test pic_intent_race
run_test "canic-tests pic_sharding_bootstrap" -p canic-tests --test pic_sharding_bootstrap
run_test "canic-tests pic_role_attestation" -p canic-tests --test pic_role_attestation
run_test "canic-tests lifecycle_boundary" -p canic-tests --test lifecycle_boundary
run_test "canic-tests root_suite" -p canic-tests --test root_suite
run_test "canic-tests root_wasm_store_reconcile" -p canic-tests --test root_wasm_store_reconcile
run_test "canic-tests instruction_audit" -p canic-tests --test instruction_audit

print_summary
