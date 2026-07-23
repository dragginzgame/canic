#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CANIC_BIN="${CANIC_BIN:-$ROOT/target/debug/canic}"
PROOF_ROOT="${1:-$(mktemp -d "${TMPDIR:-/tmp}/canic-v1-operator-proof.XXXXXX")}"
DEPLOYMENT="demo-local"
ROOT_PRINCIPAL="uxrrr-q7777-77774-qaaaq-cai"
BUILD_PROVENANCE="$PROOF_ROOT/build-provenance.json"
DEPLOYMENT_CHECK="$PROOF_ROOT/deployment-check-envelope.json"

require_canic_bin() {
    if [ ! -x "$CANIC_BIN" ]; then
        echo "missing canic binary at $CANIC_BIN" >&2
        echo "run: cargo build -p canic-cli" >&2
        echo "or set CANIC_BIN=/path/to/canic" >&2
        exit 2
    fi
}

prepare_proof_root() {
    if [ -e "$PROOF_ROOT" ] && [ -n "$(ls -A "$PROOF_ROOT")" ]; then
        echo "proof output directory is not empty: $PROOF_ROOT" >&2
        exit 2
    fi

    mkdir -p "$PROOF_ROOT/apps/demo"
    cp "$ROOT/apps/demo/canic.toml" "$PROOF_ROOT/apps/demo/canic.toml"
}

assert_contains() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "expected $path to contain: $needle" >&2
        echo "actual content:" >&2
        sed -n '1,180p' "$path" >&2
        exit 1
    fi
}

main() {
    require_canic_bin
    prepare_proof_root

    cd "$ROOT"
    "$CANIC_BIN" build demo app \
        --profile fast \
        --icp-root "$PROOF_ROOT" \
        --provenance "$BUILD_PROVENANCE" > "$PROOF_ROOT/build.out"

    cd "$PROOF_ROOT"
    "$CANIC_BIN" deploy register "$DEPLOYMENT" \
        --fleet-template demo \
        --root "$ROOT_PRINCIPAL" \
        --allow-unverified > register.out

    set +e
    "$CANIC_BIN" deploy check "$DEPLOYMENT" \
        --format envelope-json \
        --build-provenance "$BUILD_PROVENANCE" > "$DEPLOYMENT_CHECK"
    check_status=$?
    set -e

    assert_contains "$BUILD_PROVENANCE" '"id": "canic.build_provenance.v1"'
    assert_contains "$BUILD_PROVENANCE" '"kind": "artifact"'
    assert_contains "$BUILD_PROVENANCE" '"app": "demo"'
    assert_contains "$BUILD_PROVENANCE" '"role": "app"'
    assert_contains "$BUILD_PROVENANCE" '"build_status": "success"'
    assert_contains "$BUILD_PROVENANCE" '"artifact_kind": "wasm_gzip"'
    assert_contains "$DEPLOYMENT_CHECK" '"id": "canic.deployment_check.v1"'
    assert_contains "$DEPLOYMENT_CHECK" '"kind": "deployment"'
    assert_contains "$DEPLOYMENT_CHECK" "\"deployment\": \"$DEPLOYMENT\""
    assert_contains "$DEPLOYMENT_CHECK" '"kind": "build_provenance"'
    assert_contains "$DEPLOYMENT_CHECK" '"id": "canic.build_provenance.v1"'
    assert_contains "$DEPLOYMENT_CHECK" '"exit_class": "blocked_by_policy"'
    if [ "$check_status" -eq 0 ]; then
        echo "expected deploy check to return a blocked status for this proof" >&2
        exit 1
    fi

    echo "v1 operator proof passed"
    echo "proof root: $PROOF_ROOT"
    echo "build provenance: $BUILD_PROVENANCE"
    echo "deployment check: $DEPLOYMENT_CHECK"
}

main "$@"
