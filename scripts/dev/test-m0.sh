#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/m0
case "${1:-all}" in
    all) tests=(--test creation --test funding --test maintenance) ;;
    maintenance) tests=(--test maintenance) ;;
    *) printf 'Usage: %s [all|maintenance]\n' "$0" >&2; exit 2 ;;
esac
for variant in original maintenance-candidate; do
    features=''
    if [ "$variant" = maintenance-candidate ]; then features=maintenance-candidate; fi
    records=$(mktemp target/m0/build.XXXXXX.jsonl)
    if ! cargo build --locked --manifest-path qualification/Cargo.toml \
        -p canic-m0-probe --target wasm32-unknown-unknown --release --features "$features" \
        --message-format=json-render-diagnostics > "$records"; then
        jq -r 'select(.reason == "compiler-message") | .message.rendered // empty' "$records" >&2
        exit 1
    fi
    wasm=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic_m0_probe")
        | .filenames[] | select(endswith(".wasm"))] | unique |
        if length == 1 then .[0] else error("expected one probe Wasm artifact") end' "$records")
    cp -- "$wasm" "target/m0/$variant.wasm"
done
cargo test --locked --manifest-path qualification/Cargo.toml -p canic-m0-probe --lib
CANIC_M0_PROBE_WASM="$root/target/m0/original.wasm" \
    CANIC_M0_CANDIDATE_WASM="$root/target/m0/maintenance-candidate.wasm" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-m0-tests "${tests[@]}" -- --nocapture
