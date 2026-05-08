#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-installed-cli.XXXXXX")"
INSTALL_ROOT="$TMP_ROOT/install-root"
BIN_ROOT="$INSTALL_ROOT/bin"
SPLIT_ICP_ROOT="$TMP_ROOT/split-icp-root"

source "$ROOT/scripts/ci/require_icp.sh"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

main() {
    require_icp_tools

    cargo install --offline --locked --path "$ROOT/crates/canic-cli" --root "$INSTALL_ROOT" >/dev/null

    (
        cd "$ROOT"
        CANIC_WORKSPACE_ROOT="$ROOT" CANIC_WASM_PROFILE=fast ICP_ENVIRONMENT=local \
            "$BIN_ROOT/canic" build app >/dev/null
        CANIC_WORKSPACE_ROOT="$ROOT" CANIC_WASM_PROFILE=fast ICP_ENVIRONMENT=local \
            "$BIN_ROOT/canic" build root >/dev/null
    )

    [ -s "$ROOT/.icp/local/canisters/app/app.wasm.gz" ] || {
        echo "expected installed builder to emit app.wasm.gz" >&2
        exit 1
    }

    [ -s "$ROOT/.icp/local/canisters/root/root.wasm.gz" ] || {
        echo "expected installed builder to emit root.wasm.gz" >&2
        exit 1
    }

    [ -s "$ROOT/.icp/local/canisters/root/root.release-set.json" ] || {
        echo "expected installed canic CLI to emit root.release-set.json" >&2
        exit 1
    }

    mkdir -p "$SPLIT_ICP_ROOT"
    (
        cd "$ROOT"
        CANIC_WORKSPACE_ROOT="$ROOT" CANIC_ICP_ROOT="$SPLIT_ICP_ROOT" CANIC_WASM_PROFILE=fast ICP_ENVIRONMENT=local \
            "$BIN_ROOT/canic" build root >/dev/null
    )

    [ -s "$SPLIT_ICP_ROOT/.icp/local/canisters/wasm_store/wasm_store.wasm.gz" ] || {
        echo "expected split-root probe to emit wasm_store.wasm.gz under CANIC_ICP_ROOT" >&2
        exit 1
    }

    [ -s "$SPLIT_ICP_ROOT/.icp/local/canisters/root/root.wasm.gz" ] || {
        echo "expected split-root probe to emit root.wasm.gz under CANIC_ICP_ROOT" >&2
        exit 1
    }

    echo "installed canic CLI probe passed"
}

main "$@"
