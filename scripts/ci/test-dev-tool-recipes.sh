#!/usr/bin/env bash
set -euo pipefail

# Exercise the real Make recipes without installing tools or changing Git state.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-dev-tool-recipes.XXXXXX")"
fixture_complete=false
finish() {
    local status=$?
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture"
    else printf 'Failed development tool fixture retained: %s\n' "$fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
real_bash="$(command -v bash)"
real_make="$(command -v make)"
mkdir -p "$fixture/.tools/rust/fixture-testkit/installed/bin" "$fixture/bin" "$fixture/cargo tools" "$fixture/.tools/ic/bin" \
    "$fixture/.tools/host/bin" "$fixture/make" "$fixture/ci" "$fixture/scripts/ci"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$fixture/"
cp "$ROOT/make/tools.mk" "$ROOT/make/execution.mk" "$ROOT/make/release.mk" "$ROOT/make/consumer.mk" "$ROOT/make/rust-format.mk" "$fixture/make/"
cp "$ROOT/scripts/ci/check-make-execution.sh" "$fixture/scripts/ci/"
cp "$ROOT/ci/ic-tools.tsv" "$ROOT/ci/tool-versions.env" "$fixture/ci/"
cp "$ROOT/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
export EVENTS="$fixture/events" INSTALL_SELECTION="$fixture/install-selection"
export CANIC_RECIPE_TEST_BASH="$real_bash"

cat >"$fixture/record" <<'SH'
set -euo pipefail
if [[ "$1" == */check-make-execution.sh ]]; then exec "$CANIC_RECIPE_TEST_BASH" "$@"; fi
if [[ -f "$1" && ( "$1" == scripts/dev/candid-extractor.sh || "$1" == */scripts/dev/install-rust-tools.sh ) ]]; then
    exec "$CANIC_RECIPE_TEST_BASH" "$@"
fi
name="${0##*/}"
printf '%s\t%s\n' "$0" "$*" >>"$EVENTS"
case "$name:$*" in
    'bash:scripts/dev/install_dev.sh' | 'bash:scripts/dev/install_dev.sh --update-prereqs')
        printf '%s\n' "$PWD/.tools/ic/bin" >"$INSTALL_SELECTION"
        ;;
    'bash:scripts/ci/testkit-server.sh' | 'bash:scripts/ci/testkit-server.sh --check')
        printf '%s\n' "$PWD/.tools/rust/fixture-testkit/installed/bin/testkit-runner"
        ;;
    'bash:scripts/dev/candid-extractor.sh' | 'bash:scripts/dev/candid-extractor.sh --check')
        exit "${FAIL_EXTRACTOR:-0}"
        ;;
    'bash:'*'/scripts/dev/install-host-tools.sh '*) exit "${FAIL_HOST_TOOLS:-0}" ;;
    'bash:'*'/scripts/dev/install-ic-tools.sh '*' --preflight') exit "${FAIL_PREFLIGHT:-0}" ;;
    'bash:'*'/scripts/dev/install-rust-tools.sh '*' --preflight') exit "${FAIL_PREFLIGHT:-0}" ;;
    'bash:'*'/scripts/dev/install-ic-tools.sh '*) exit "${FAIL_IC_TOOLS:-0}" ;;
    'bash:'*'/scripts/dev/install-rust-tools.sh '*) exit "${FAIL_RUST_TOOLS:-0}" ;;
    'testkit-runner:setup' | 'testkit-runner:check') exit "${FAIL_TESTKIT:-0}" ;;
    'wasm-opt:--version') exit "${FAIL_WASM_OPT:-0}" ;;
esac
SH
for executable in bin/bash bin/cargo '.tools/host/bin/rg' 'cargo tools/sccache' \
    '.tools/ic/bin/icp' '.tools/ic/bin/ic-wasm' '.tools/ic/bin/wasm-opt' '.tools/rust/fixture-testkit/installed/bin/testkit-runner'; do
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
    grep -Fxq "$fixture/.tools/rust/fixture-testkit/installed/bin/testkit-runner"$'\t'"$action" "$EVENTS"
    if FAIL_TESTKIT=17 run_recipe "$target"; then
        echo 'Testkit recipe accepted a failed owner command' >&2
        exit 1
    fi
done
# The actual aggregate preserves common/product ordering under parallel Make.
for target in install-tools tools-check; do
    suffix=''; action=setup
    if [[ "$target" == tools-check ]]; then suffix=' --check'; action=check; fi
    run_recipe -j8 "$target"
    {
        if [[ "$target" == install-tools ]]; then
            printf '%s\t%s\n' "$fixture/bin/bash" "$fixture/scripts/dev/install-rust-tools.sh --consumer $fixture --versions $fixture/ci/tool-versions.env --preflight"
        fi
        printf '%s\t%s\n' "$fixture/bin/bash" "$fixture/scripts/dev/install-host-tools.sh --consumer $fixture --versions $fixture/ci/tool-versions.env$suffix"
        printf '%s\t%s\n' "$fixture/bin/bash" "$fixture/scripts/dev/install-rust-tools.sh --consumer $fixture --versions $fixture/ci/tool-versions.env$suffix"
        printf '%s\t%s\n' "$fixture/bin/bash" "$fixture/scripts/dev/install-ic-tools.sh --consumer $fixture --pins $fixture/ci/ic-tools.tsv$suffix"
        printf '%s\t%s\n' "$fixture/bin/bash" "scripts/dev/candid-extractor.sh$suffix"
        printf '%s\t%s\n' "$fixture/bin/bash" "scripts/ci/testkit-server.sh$suffix"
        printf '%s\t%s\n' "$fixture/.tools/rust/fixture-testkit/installed/bin/testkit-runner" "$action"
    } > "$fixture/aggregate-expected"
    cmp "$fixture/aggregate-expected" "$EVENTS"
    if FAIL_HOST_TOOLS=17 run_recipe -j8 "$target"; then
        echo 'aggregate accepted failed host admission' >&2; exit 1
    fi
    expected_prefix=0
    if [[ "$target" == install-tools ]]; then expected_prefix=1; fi
    [[ "$(wc -l < "$EVENTS")" == $((expected_prefix + 1)) ]] || exit 1
    if FAIL_RUST_TOOLS=17 run_recipe -j8 "$target"; then
        echo 'aggregate accepted failed Cargo-tool admission' >&2; exit 1
    fi
    [[ "$(wc -l < "$EVENTS")" == $((expected_prefix + 2)) ]] || exit 1
    if FAIL_EXTRACTOR=17 run_recipe -j8 "$target"; then
        echo 'aggregate accepted failed extractor admission' >&2; exit 1
    fi
    [[ "$(wc -l < "$EVENTS")" == $((expected_prefix + 4)) ]] || exit 1
    if FAIL_TESTKIT=17 run_recipe -j8 "$target"; then
        echo 'aggregate accepted failed product admission' >&2; exit 1
    fi
done
if FAIL_PREFLIGHT=17 run_recipe -j8 install-tools; then
    echo 'aggregate accepted failed preflight' >&2; exit 1
fi
[[ "$(wc -l < "$EVENTS")" == 1 ]] || exit 1

# Canic supplies the root lock; Shared Tooling owns package admission and drift.
mkdir -p "$fixture/scripts/dev"
cp "$ROOT/scripts/ci/testkit-server.sh" "$fixture/scripts/ci/"
cat > "$fixture/scripts/dev/install-rust-tools.sh" <<'SELECT'
set -euo pipefail
[[ "$1" == --consumer && "$2" == "$PWD" && "$3" == --package && "$4" == ic-testkit ]] || exit 1
[[ "$5" == --lockfile && "$6" == "$PWD/Cargo.lock" ]] || exit 1
[[ "$7" == --bin && "$8" == ic-testkit-server ]] || exit 1
[[ "$9" == --profile && "${10}" == release ]] || exit 1
[[ $# == 10 || ( $# == 11 && "${11}" == --check ) ]] || exit 1
selected_version=$(yq -p toml -o json -I 0 '.' "$6" | jq -r '.package[] | select(.name == "ic-testkit") | .version')
printf '%s\n' "$selected_version ${11:-setup}" >> "$INSTALL_SELECTION"
printf '%s\n' "$PWD/.tools/rust/fixture-testkit/installed/bin/testkit-runner"
SELECT
: > "$INSTALL_SELECTION"
for version in 99.1.0 99.2.0; do
    printf 'version = 4\n[[package]]\nname = "ic-testkit"\nversion = "%s"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n' "$version" > Cargo.lock
    "$real_bash" scripts/ci/testkit-server.sh > "$fixture/selected-runner"
    "$real_bash" scripts/ci/testkit-server.sh --check > "$fixture/checked-runner"
    cmp "$fixture/selected-runner" "$fixture/checked-runner"
done
printf '99.1.0 setup\n99.1.0 --check\n99.2.0 setup\n99.2.0 --check\n' > "$fixture/expected-versions"
cmp "$fixture/expected-versions" "$INSTALL_SELECTION"
for unsupported in --version --package --lockfile; do
    if "$real_bash" scripts/ci/testkit-server.sh "$unsupported" > "$fixture/unsupported.log" 2>&1; then
        echo "Testkit adapter accepted $unsupported override" >&2; exit 1
    fi
    cmp "$fixture/expected-versions" "$INSTALL_SELECTION"
done
# Exercise the actual extractor adapter independently of the common Rust set.
cp "$ROOT/scripts/dev/candid-extractor.sh" scripts/dev/
cat > scripts/dev/install-rust-tools.sh <<'SELECT'
set -euo pipefail
printf '%s\0' "$@" > "$INSTALL_SELECTION"
[[ "${CANIC_EXTRACTOR_FAILURE:-0}" == 0 ]] || exit "$CANIC_EXTRACTOR_FAILURE"
printf '%s\n' "$PWD/admitted bin/candid-extractor"
SELECT
for suffix in '' --check; do
    arguments=()
    [[ -z "$suffix" ]] || arguments+=("$suffix")
    "$real_bash" scripts/dev/candid-extractor.sh ${arguments[@]+"${arguments[@]}"} > selected-extractor
    printf '%s\0' --consumer "$fixture" --package candid-extractor --version 0.1.6 \
        --bin candid-extractor --profile release ${arguments[@]+"${arguments[@]}"} > expected-extractor
    cmp expected-extractor "$INSTALL_SELECTION"
    [[ "$(cat selected-extractor)" == "$fixture/admitted bin/candid-extractor" ]] || exit 1
    status=0
    CANIC_EXTRACTOR_FAILURE=37 "$real_bash" scripts/dev/candid-extractor.sh ${arguments[@]+"${arguments[@]}"} > selected-extractor || status=$?
    [[ "$status" == 37 && ! -s selected-extractor ]] || exit 1
done
cp "$INSTALL_SELECTION" expected-extractor
for override in --version --package --lockfile; do
    status=0
    "$real_bash" scripts/dev/candid-extractor.sh "$override" > selected-extractor 2>/dev/null || status=$?
    [[ "$status" == 2 && ! -s selected-extractor ]] || exit 1
    cmp expected-extractor "$INSTALL_SELECTION"
done
# Execute both maintained native setup bodies. Other installers are substitutes;
# Make/adapter dispatch and GitHub PATH publication use the actual caller code.
export CANIC_EXTRACTOR_FIXTURE="$fixture" CANIC_EXTRACTOR_MAKE="$real_make"
printf '#!%s\n' "$real_bash" > bin/make
cat >> bin/make <<'MAKE'
set -euo pipefail
case "$*" in
    'install-rust-tools install-candid-extractor')
        exec "$CANIC_EXTRACTOR_MAKE" --no-print-directory -C "$CANIC_EXTRACTOR_FIXTURE" install-candid-extractor ;;
    '--no-print-directory candid-extractor-check')
        exec "$CANIC_EXTRACTOR_MAKE" --no-print-directory -C "$CANIC_EXTRACTOR_FIXTURE" candid-extractor-check ;;
    install-host-tools|'install-ic-tools install-testkit-server'|testkit-server-check) exit 0 ;;
    *) exit 97 ;;
esac
MAKE
chmod +x bin/make
export GITHUB_WORKSPACE="$fixture" GITHUB_PATH="$fixture/github-path"
for step in 'Install and qualify pinned native build tools' 'Install PocketIC test helper tools'; do
    yq -o json '.' "$ROOT/.github/workflows/ci.yml" | jq -er --arg step "$step" \
        '[.jobs[].steps[] | select(.name == $step) | .run] | if length == 1 then .[0] else error("ambiguous native setup") end' > workflow-step
    : > "$GITHUB_PATH"
    PATH="$fixture/bin:$PATH" "$real_bash" -e -o pipefail workflow-step > workflow-output
    grep -Fx "$fixture/admitted bin" "$GITHUB_PATH" >/dev/null
    : > "$GITHUB_PATH"
    status=0
    CANIC_EXTRACTOR_FAILURE=37 PATH="$fixture/bin:$PATH" "$real_bash" -e -o pipefail workflow-step > workflow-output 2>&1 || status=$?
    [[ "$status" != 0 ]] || exit 1
    if grep -Fx "$fixture/admitted bin" "$GITHUB_PATH"; then exit 1; fi
done
echo 'Development recipes, explicit extractor admission and native workflow PATH publication passed (installer effects substituted)'
fixture_complete=true
