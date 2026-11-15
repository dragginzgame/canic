#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/effects
case "${1:-all}" in
    all) tests=(--test creation --test funding --test maintenance) ;;
    maintenance) tests=(--test maintenance) ;;
    *) printf 'Usage: %s [all|maintenance]\n' "$0" >&2; exit 2 ;;
esac
for variant in original maintenance-candidate; do
    features=''
    if [ "$variant" = maintenance-candidate ]; then features=maintenance-candidate; fi
    records=$(mktemp target/effects/build.XXXXXX.jsonl)
    if ! cargo build --locked --manifest-path qualification/Cargo.toml \
        -p canic-effect-probe --target wasm32-unknown-unknown --release --features "$features" \
        --message-format=json-render-diagnostics > "$records"; then
        jq -r 'select(.reason == "compiler-message") | .message.rendered // empty' "$records" >&2
        exit 1
    fi
    wasm=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic_effect_probe")
        | .filenames[] | select(endswith(".wasm"))] | unique |
        if length == 1 then .[0] else error("expected one probe Wasm artifact") end' "$records")
    cp -- "$wasm" "target/effects/$variant.wasm"
done
cargo test --locked --manifest-path qualification/Cargo.toml -p canic-effect-probe --lib
CANIC_EFFECT_PROBE_WASM="$root/target/effects/original.wasm" \
    CANIC_EFFECT_CANDIDATE_WASM="$root/target/effects/maintenance-candidate.wasm" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification "${tests[@]}" -- --nocapture
