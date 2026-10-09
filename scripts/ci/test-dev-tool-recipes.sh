#!/usr/bin/env bash
set -euo pipefail

# Exercise the real Make recipes without installing tools or changing Git state.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-dev-tool-recipes.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
real_bash="$(command -v bash)"
real_make="$(command -v make)"
mkdir -p "$fixture/.tools/rust/bin" "$fixture/bin" "$fixture/cargo tools" "$fixture/.tools/ic/bin" \
    "$fixture/.tools/host/bin" "$fixture/make" "$fixture/ci" "$fixture/scripts/ci"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$fixture/"
cp "$ROOT/make/tools.mk" "$fixture/make/"
cp "$ROOT/ci/ic-tools.tsv" "$ROOT/ci/tool-versions.env" "$fixture/ci/"
cp "$ROOT/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
export EVENTS="$fixture/events" INSTALL_SELECTION="$fixture/install-selection"

cat >"$fixture/record" <<'SH'
set -euo pipefail
name="${0##*/}"
printf '%s\t%s\n' "$0" "$*" >>"$EVENTS"
case "$name:$*" in
    'bash:scripts/dev/install_dev.sh' | 'bash:scripts/dev/install_dev.sh --update-prereqs')
        printf '%s\n' "$PWD/.tools/ic/bin" >"$INSTALL_SELECTION"
        ;;
    'bash:scripts/ci/testkit-server.sh' | 'bash:scripts/ci/testkit-server.sh --check')
        printf '%s\n' "$PWD/.tools/rust/bin/testkit-runner"
        ;;
    'testkit-runner:setup' | 'testkit-runner:check') exit "${FAIL_TESTKIT:-0}" ;;
    'wasm-opt:--version') exit "${FAIL_WASM_OPT:-0}" ;;
esac
SH
for executable in bin/bash bin/cargo '.tools/host/bin/rg' 'cargo tools/sccache' \
    '.tools/ic/bin/icp' '.tools/ic/bin/ic-wasm' '.tools/ic/bin/wasm-opt' '.tools/rust/bin/testkit-runner'; do
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
    PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory \
        CARGO_INSTALL_BIN_DIR="$fixture/cargo tools" \
        "$@" >"$fixture/output" 2>&1
}

printf '%s\n' "$fixture/.tools/ic/bin" >"$fixture/expected-selection"
for target in install-dev update-dev; do
    if ! run_recipe "$target"; then
        cat "$fixture/output" >&2
        exit 1
    fi
    cmp "$fixture/expected-selection" "$INSTALL_SELECTION"
done
for executable in '.tools/ic/bin/icp' '.tools/ic/bin/ic-wasm' '.tools/ic/bin/wasm-opt'; do
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
for target in install-testkit-server testkit-server-check; do
    run_recipe "$target"
    if [[ "$target" == install-testkit-server ]]; then action=setup; else action=check; fi
    grep -Fxq "$fixture/.tools/rust/bin/testkit-runner"$'\t'"$action" "$EVENTS"
    if FAIL_TESTKIT=17 run_recipe "$target"; then
        echo 'Testkit recipe accepted a failed owner command' >&2
        exit 1
    fi
done
echo 'Development Make recipes: configured paths, spaces, PATH shadowing and failure propagation passed'
