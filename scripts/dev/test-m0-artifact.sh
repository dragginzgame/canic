#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/m0/artifact

# Cargo identifies each executable; the separate copies survive feature rebuilds.
build_wasm() {
    local manifest=$1 package=$2 target_name=$3 destination=$4 features=$5 records wasm
    records=$(mktemp target/m0/artifact/build.XXXXXX.jsonl)
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

build_wasm qualification/Cargo.toml canic-m0-artifact canic_m0_artifact \
    target/m0/artifact/original.wasm ''
build_wasm qualification/Cargo.toml canic-m0-artifact canic_m0_artifact \
    target/m0/artifact/candidate.wasm maintenance-candidate
build_wasm Cargo.toml canic-hello canic_hello target/m0/artifact/worker.wasm ''
cargo test --locked --manifest-path qualification/Cargo.toml -p canic-m0-artifact --lib -- --nocapture
CANIC_M0_PROBE_WASM="$root/target/m0/artifact/original.wasm" \
    CANIC_M0_ARTIFACT_DIRECTORY="$root/target/m0/artifact" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-m0-tests --test artifact -- --nocapture
