#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/counter
records=$(mktemp target/counter/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
for variant in v1 v2 reject incompatible; do
    features=()
    case "$variant" in
        v2) features=(--features counter-v2) ;;
        reject) features=(--features reject-upgrade) ;;
        incompatible) features=(--features incompatible-schema) ;;
    esac
    "$cli" artifact build --json --package canic-demo "${features[@]}" --output "target/counter/$variant.wasm" > "target/counter/$variant.json"
done
CANIC_COUNTER_ARTIFACTS="$root/target/counter" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test counter -- --nocapture
