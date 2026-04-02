#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-installed-installer.XXXXXX")"
INSTALL_ROOT="$TMP_ROOT/install-root"
BIN_ROOT="$INSTALL_ROOT/bin"
SPLIT_DFX_ROOT="$TMP_ROOT/split-dfx-root"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

main() {
    cargo install --offline --locked --path "$ROOT/crates/canic-installer" --root "$INSTALL_ROOT" >/dev/null

    (
        cd "$ROOT"
        CANIC_WORKSPACE_ROOT="$ROOT" CANIC_WASM_PROFILE=fast DFX_NETWORK=local \
            "$BIN_ROOT/canic-build-canister-artifact" app >/dev/null
        CANIC_WORKSPACE_ROOT="$ROOT" CANIC_WASM_PROFILE=fast DFX_NETWORK=local \
            "$BIN_ROOT/canic-build-canister-artifact" root >/dev/null
        CANIC_WORKSPACE_ROOT="$ROOT" DFX_NETWORK=local \
            "$BIN_ROOT/canic-emit-root-release-set-manifest" --if-ready >/dev/null
    )

    [ -s "$ROOT/.dfx/local/canisters/app/app.wasm.gz" ] || {
        echo "expected installed builder to emit app.wasm.gz" >&2
        exit 1
    }

    [ -s "$ROOT/.dfx/local/canisters/root/root.wasm.gz" ] || {
        echo "expected installed builder to emit root.wasm.gz" >&2
        exit 1
    }

    [ -s "$ROOT/.dfx/local/canisters/root/root.release-set.json" ] || {
        echo "expected installed binaries to emit root.release-set.json" >&2
        exit 1
    }

    mkdir -p "$SPLIT_DFX_ROOT"
    (
        cd "$ROOT"
        CANIC_WORKSPACE_ROOT="$ROOT" CANIC_DFX_ROOT="$SPLIT_DFX_ROOT" CANIC_WASM_PROFILE=fast DFX_NETWORK=local \
            "$BIN_ROOT/canic-build-canister-artifact" root >/dev/null
    )

    [ -s "$SPLIT_DFX_ROOT/.dfx/local/canisters/wasm_store/wasm_store.wasm.gz" ] || {
        echo "expected split-root probe to emit wasm_store.wasm.gz under CANIC_DFX_ROOT" >&2
        exit 1
    }

    [ -s "$SPLIT_DFX_ROOT/.dfx/local/canisters/root/root.wasm.gz" ] || {
        echo "expected split-root probe to emit root.wasm.gz under CANIC_DFX_ROOT" >&2
        exit 1
    }

    echo "installed canic-installer probe passed"
}

main "$@"
