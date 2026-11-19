#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/artifact-custody

# Cargo identifies each executable; the separate copies survive feature rebuilds.
build_wasm() {
    local manifest=$1 package=$2 target_name=$3 destination=$4 features=$5 records wasm
    records=$(mktemp target/artifact-custody/build.XXXXXX.jsonl)
    if ! cargo build --locked --manifest-path "$manifest" -p "$package" \
        --target wasm32-unknown-unknown --release --features "$features" \
        --message-format=json-render-diagnostics > "$records"; then
        jq -r 'select(.reason == "compiler-message") | .message.rendered // empty' "$records" >&2
        return 1
    fi
    wasm=$(jq -sr --arg name "$target_name" '[.[] |
        select(.reason == "compiler-artifact" and .target.name == $name) |
        .filenames[] | select(endswith(".wasm"))] | unique |
        if length == 1 then .[0] else error("expected one Wasm artifact") end' "$records")
    cp -- "$wasm" "$destination"
}

build_wasm qualification/Cargo.toml canic-artifact-probe canic_artifact_probe \
    target/artifact-custody/original.wasm ''
build_wasm qualification/Cargo.toml canic-artifact-probe canic_artifact_probe \
    target/artifact-custody/candidate.wasm maintenance-candidate
build_wasm Cargo.toml ex-greeting ex_greeting target/artifact-custody/worker.wasm ''
cargo test --locked --manifest-path qualification/Cargo.toml -p canic-artifact-probe --lib -- --nocapture
CANIC_EFFECT_PROBE_WASM="$root/target/artifact-custody/original.wasm" \
    CANIC_EFFECT_ARTIFACT_DIRECTORY="$root/target/artifact-custody" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification --test artifact -- --nocapture
