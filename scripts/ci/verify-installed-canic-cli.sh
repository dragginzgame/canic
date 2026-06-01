#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-installed-cli.XXXXXX")"
INSTALL_ROOT="$TMP_ROOT/install-root"
BIN_ROOT="$INSTALL_ROOT/bin"
PROOF_HOME="$TMP_ROOT/home"
PROOF_CARGO_HOME="$TMP_ROOT/cargo-home"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

main() {
    cargo install --offline --locked --path "$ROOT/crates/canic-cli" --root "$INSTALL_ROOT" >/dev/null

    mkdir -p "$PROOF_HOME" "$PROOF_CARGO_HOME"

    HOME="$PROOF_HOME" \
        CARGO_HOME="$PROOF_CARGO_HOME" \
        CANIC_BIN="$BIN_ROOT/canic" \
        "$ROOT/scripts/ci/v1-readiness-smoke.sh" > "$TMP_ROOT/v1-readiness-smoke.out"

    grep -q 'v1 readiness smoke passed' "$TMP_ROOT/v1-readiness-smoke.out" || {
        echo "expected installed canic CLI to pass v1 readiness smoke" >&2
        sed -n '1,160p' "$TMP_ROOT/v1-readiness-smoke.out" >&2
        exit 1
    }

    echo "installed canic CLI probe passed"
}

main "$@"
