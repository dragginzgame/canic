#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-full}"
HARNESS_ARGS=(-- --test-threads=1 --nocapture)

elapsed_seconds() {
    local started_at="$1"
    echo "$((SECONDS - started_at))s"
}

run_test() {
    local label="$1"
    shift
    echo "==> $label"
    local started_at="$SECONDS"
    cargo test "$@" "${HARNESS_ARGS[@]}"
    echo "==> $label done in $(elapsed_seconds "$started_at")"
}

prebuild_root_test_artifacts() {
    local label="prebuild local DFX artifacts for PocketIC root suites"
    echo "==> $label"
    local started_at="$SECONDS"
    bash scripts/ci/build-ci-wasm-artifacts.sh
    echo "==> $label done in $(elapsed_seconds "$started_at")"
}

# Compile and run all unit/lib/bin tests together first.
run_test "workspace lib/bin tests" --workspace --lib --bins

if [[ "$MODE" == "fast" ]]; then
    exit 0
fi

# Keep non-PocketIC integration tests explicit so the heavy PocketIC suites can
# run in a deterministic order without sitting behind a shared runtime lock.
run_test "canic control_plane_facade" -p canic --test control_plane_facade
run_test "canic workspace_manifest" -p canic --test workspace_manifest
run_test "canic-core trap_guard" -p canic-core --test trap_guard

# PocketIC-backed integration suites.
prebuild_root_test_artifacts
run_test "canic-core pic_intent_race" -p canic-core --test pic_intent_race
run_test "canic-core pic_sharding_bootstrap" -p canic-core --test pic_sharding_bootstrap
run_test "canic-core pic_role_attestation" -p canic-core --test pic_role_attestation
run_test "canic-tests delegation_flow" -p canic-tests --test delegation_flow
run_test "canic-tests lifecycle_boundary" -p canic-tests --test lifecycle_boundary
run_test "canic-tests root_suite" -p canic-tests --test root_suite
run_test "canic-tests root_wasm_store_reconcile" -p canic-tests --test root_wasm_store_reconcile
run_test "canic-tests instruction_audit" -p canic-tests --test instruction_audit
