#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-installed-cli.XXXXXX")"
INSTALL_ROOT="$TMP_ROOT/install-root"
BIN_ROOT="$INSTALL_ROOT/bin"
PROOF_HOME="$TMP_ROOT/home"
PROOF_CARGO_HOME="$TMP_ROOT/cargo-home"
PROOF_TARGET_DIR="$TMP_ROOT/cargo-target"
PROOF_TMPDIR="$TMP_ROOT/tmp"
DOWNSTREAM_ROOT="$TMP_ROOT/downstream-root"
FAKE_ICP="$TMP_ROOT/fake-icp"
FAKE_ICP_STATE="$TMP_ROOT/fake-icp-state"
SMOKE_OUTPUT="$TMP_ROOT/v1-readiness-smoke.out"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

. "$ROOT/scripts/ci/blob-storage-cli-proof-lib.sh"
. "$ROOT/scripts/ci/auth-renewal-cli-proof-lib.sh"

assert_installed_binary_path() {
    local canic_bin="$1"
    case "$canic_bin" in
        "$ROOT"/target/*)
            echo "installed CLI proof must not use repository target binary: $canic_bin" >&2
            exit 1
            ;;
    esac

    if [ "$canic_bin" != "$BIN_ROOT/canic" ]; then
        echo "installed CLI proof expected temp installed binary at $BIN_ROOT/canic" >&2
        echo "actual binary: $canic_bin" >&2
        exit 1
    fi
}

run_installed_canic() {
    HOME="$PROOF_HOME" \
        CARGO_HOME="$PROOF_CARGO_HOME" \
        CARGO_TARGET_DIR="$PROOF_TARGET_DIR" \
        TMPDIR="$PROOF_TMPDIR" \
        "$BIN_ROOT/canic" "$@"
}

run_installed_canic_in_workspace() {
    (
        cd "$DOWNSTREAM_ROOT"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$PROOF_CARGO_HOME" \
            CARGO_TARGET_DIR="$PROOF_TARGET_DIR" \
            TMPDIR="$PROOF_TMPDIR" \
            FAKE_ICP_STATE="$FAKE_ICP_STATE" \
            "$BIN_ROOT/canic" "$@"
    )
}

prepare_blob_storage_workspace() {
    mkdir -p \
        "$DOWNSTREAM_ROOT/fleets/downstream/app" \
        "$DOWNSTREAM_ROOT/fleets/downstream/root"

    cat > "$DOWNSTREAM_ROOT/Cargo.toml" <<'EOF'
[workspace]
members = []
resolver = "2"

[workspace.package]
version = "0.0.0"
EOF

    cat > "$DOWNSTREAM_ROOT/fleets/downstream/canic.toml" <<'EOF'
controllers = []
app_index = ["app"]

[fleet]
name = "downstream"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
EOF

    prepare_blob_storage_cli_fixture "$DOWNSTREAM_ROOT"
}

main() {
    cargo install --offline --locked --path "$ROOT/crates/canic-cli" --root "$INSTALL_ROOT" >/dev/null

    mkdir -p "$PROOF_HOME" "$PROOF_CARGO_HOME" "$PROOF_TARGET_DIR" "$PROOF_TMPDIR"
    assert_installed_binary_path "$BIN_ROOT/canic"

    HOME="$PROOF_HOME" \
        CARGO_HOME="$PROOF_CARGO_HOME" \
        CARGO_TARGET_DIR="$PROOF_TARGET_DIR" \
        TMPDIR="$PROOF_TMPDIR" \
        CANIC_BIN="$BIN_ROOT/canic" \
        "$ROOT/scripts/ci/v1-readiness-smoke.sh" > "$SMOKE_OUTPUT"

    grep -q 'v1 readiness smoke passed' "$SMOKE_OUTPUT" || {
        echo "expected installed canic CLI to pass v1 readiness smoke" >&2
        sed -n '1,160p' "$SMOKE_OUTPUT" >&2
        exit 1
    }

    run_installed_canic blob-storage --help > "$TMP_ROOT/blob-storage-help.out"
    if run_installed_canic blob-storage status downstream app --json \
        > "$TMP_ROOT/blob-storage-status-json.out" \
        2> "$TMP_ROOT/blob-storage-status-json.err"
    then
        echo "expected installed blob-storage JSON status without project state to fail" >&2
        exit 1
    fi
    prepare_blob_storage_workspace
    prepare_auth_renewal_cli_surface_fixture "$DOWNSTREAM_ROOT"
    prepare_fake_blob_storage_icp "$FAKE_ICP" "$FAKE_ICP_STATE"
    run_blob_storage_cli_probe_commands run_installed_canic_in_workspace "$TMP_ROOT" "$FAKE_ICP"
    run_auth_renewal_cli_surface_probe_commands run_installed_canic_in_workspace "$TMP_ROOT" "$FAKE_ICP"

    assert_blob_storage_cli_probe_outputs "installed" "$TMP_ROOT"
    assert_auth_renewal_cli_surface_probe_outputs "installed" "$TMP_ROOT"

    echo "installed canic CLI probe passed"
}

main "$@"
