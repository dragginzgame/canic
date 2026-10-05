#!/usr/bin/env bash
set -euo pipefail

# Exercise the real Make recipes without installing tools or changing Git state.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-dev-tool-recipes.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
real_bash="$(command -v bash)"
real_make="$(command -v make)"
mkdir -p "$fixture/bin" "$fixture/cargo tools" "$fixture/wasm tools" "$fixture/binaryen tools"
cp "$ROOT/tool-versions.env" "$fixture/"
export EVENTS="$fixture/events" INSTALL_SELECTION="$fixture/install-selection"

cat >"$fixture/record" <<'SH'
set -euo pipefail
name="${0##*/}"
printf '%s\t%s\n' "$0" "$*" >>"$EVENTS"
case "$name:$*" in
    'bash:scripts/dev/install_dev.sh' | 'bash:scripts/dev/install_dev.sh --update-prereqs')
        printf '%s\n' "$BINARYEN_INSTALL_DIR" "$IC_WASM_INSTALL_DIR" >"$INSTALL_SELECTION"
        ;;
    'wasm-opt:--version') exit "${FAIL_WASM_OPT:-0}" ;;
esac
SH
for executable in bin/bash bin/cargo 'cargo tools/rg' 'cargo tools/sccache' \
    'cargo tools/icp' 'wasm tools/ic-wasm' 'binaryen tools/wasm-opt'; do
    { printf '#!%s\n' "$real_bash"; cat "$fixture/record"; } >"$fixture/$executable"
    chmod +x "$fixture/$executable"
done
# A same-named PATH entry must never replace the executable just installed.
for name in icp ic-wasm wasm-opt; do
    printf '#!/bin/sh\nexit 97\n' >"$fixture/bin/$name"
    chmod +x "$fixture/bin/$name"
done

cd "$fixture"
run_recipe() {
    : >"$EVENTS"
    rm -f "$INSTALL_SELECTION"
    PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory -f "$ROOT/Makefile" \
        CARGO_INSTALL_BIN_DIR="$fixture/cargo tools" \
        BINARYEN_INSTALL_DIR="$fixture/binaryen tools" \
        IC_WASM_INSTALL_DIR="$fixture/wasm tools" \
        "$@" >"$fixture/output" 2>&1
}

printf '%s\n' "$fixture/binaryen tools" "$fixture/wasm tools" >"$fixture/expected-selection"
for target in install-dev update-dev; do
    if ! run_recipe "$target"; then
        cat "$fixture/output" >&2
        exit 1
    fi
    cmp "$fixture/expected-selection" "$INSTALL_SELECTION"
done
for executable in 'cargo tools/icp' 'wasm tools/ic-wasm' 'binaryen tools/wasm-opt'; do
    grep -Fxq "$fixture/$executable"$'\t--version' "$EVENTS"
done

if FAIL_WASM_OPT=17 run_recipe update-dev; then
    echo 'update-dev accepted a failed installed-tool probe' >&2
    exit 1
fi
if grep -Fq 'scripts/ci/check-dependency-risk-inventory.sh' "$EVENTS"; then
    echo 'update-dev continued after a failed installed-tool probe' >&2
    exit 1
fi
echo 'Development Make recipes: configured paths, spaces, PATH shadowing and failure propagation passed'
