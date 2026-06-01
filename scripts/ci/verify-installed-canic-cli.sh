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
SMOKE_OUTPUT="$TMP_ROOT/v1-readiness-smoke.out"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

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

    echo "installed canic CLI probe passed"
}

main "$@"
