#!/usr/bin/env bash
set -euo pipefail

# Exercise actual cleanup only in a disposable workspace with marker artifacts.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-cargo-clean.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Cleanup fixture retained: $fixture" >&2; fi' EXIT
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
unset CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR
export CARGO_HOME="$fixture/cargo-home" CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0
# Compiler-cache admission is covered by the owning build-environment fixture.
export RUSTC_WRAPPER=''
workspace="$fixture/workspace with spaces"
mkdir -p "$workspace/.cargo" "$workspace/member/src" "$workspace/ci" \
    "$workspace/scripts/ci" "$fixture/retained"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$workspace/"
cp -R "$ROOT/make" "$workspace/"
cp "$ROOT/ci/tool-versions.env" "$ROOT/ci/ic-tools.tsv" "$workspace/ci/"
cp "$ROOT/scripts/ci/cleanup-release-artifacts.sh" \
    "$ROOT/scripts/ci/check-make-execution.sh" "$ROOT/scripts/ci/ic-tool-pins.sh" \
    "$workspace/scripts/ci/"
if [[ -f "$ROOT/.cargo/config.toml" ]]; then
    cp "$ROOT/.cargo/config.toml" "$workspace/.cargo/"
fi
printf '[workspace]\nmembers = ["member"]\nresolver = "3"\n' > "$workspace/Cargo.toml"
printf '[package]\nname = "cleanup-fixture"\nversion = "0.1.0"\nedition = "2024"\n' > "$workspace/member/Cargo.toml"
printf 'pub fn fixture() {}\n' > "$workspace/member/src/lib.rs"
printf 'retained\n' > "$fixture/retained/evidence"

seed_target() {
    local output="$1" directory
    for directory in debug cargo/debug canic-wasm icp-build test-artifacts; do
        mkdir -p "$output/$directory"
        printf 'artifact\n' > "$output/$directory/marker"
    done
    printf 'evidence\n' > "$output/validation-evidence.json"
    # Cargo requires this standard cache signature for explicit target selection.
    printf 'Signature: 8a477f597d28d172789f06886806bc55\n' > "$output/CACHEDIR.TAG"
}

cd "$workspace"
cargo metadata --no-deps --offline --format-version 1 > "$fixture/metadata.json"
jq -e --arg target "$workspace/target" '.target_directory == $target' \
    "$fixture/metadata.json" >/dev/null
seed_target "$workspace/target"
cargo clean
[[ ! -e "$workspace/target" ]]

# A direct operator override retains Cargo's standard selected-directory behavior.
seed_target "$workspace/target"
seed_target "$fixture/operator output"
CARGO_TARGET_DIR="$fixture/operator output" cargo clean
[[ ! -e "$fixture/operator output" && -f target/debug/marker ]]

# General Make cleanup selects the whole workspace even with an inherited child output.
mv target/CACHEDIR.TAG "$fixture/cache-tag"
if CARGO_TARGET_DIR="$workspace/target/cargo" make --no-print-directory clean \
    > "$fixture/untagged-refusal.log" 2>&1; then
    echo 'cleanup accepted an untagged explicit target' >&2
    exit 1
fi
[[ -f target/debug/marker ]]
mv "$fixture/cache-tag" target/CACHEDIR.TAG
CARGO_TARGET_DIR="$workspace/target/cargo" make --no-print-directory clean
[[ ! -e "$workspace/target" ]]
[[ -f "$fixture/retained/evidence" && -f member/src/lib.rs ]]

# Nested workspaces keep their own default compiler output.
mkdir -p nested-parent/nested/src
if [[ -f "$ROOT/integrations/blob-service/.cargo/config.toml" ]]; then
    mkdir -p nested-parent/.cargo
    cp "$ROOT/integrations/blob-service/.cargo/config.toml" nested-parent/.cargo/
fi
if [[ -f "$ROOT/integrations/blob-service/consumer/.cargo/config.toml" ]]; then
    mkdir -p nested-parent/nested/.cargo
    cp "$ROOT/integrations/blob-service/consumer/.cargo/config.toml" nested-parent/nested/.cargo/
fi
printf '[workspace]\n[package]\nname = "nested-fixture"\nversion = "0.1.0"\nedition = "2024"\n' > nested-parent/nested/Cargo.toml
printf 'pub fn nested() {}\n' > nested-parent/nested/src/lib.rs
(cd nested-parent/nested && cargo metadata --no-deps --offline --format-version 1) \
    > "$fixture/nested-metadata.json"
jq -e --arg target "$workspace/nested-parent/nested/target" '.target_directory == $target' \
    "$fixture/nested-metadata.json" >/dev/null
echo 'Workspace cleanup, explicit Cargo selection, retained external evidence and nested defaults passed'
