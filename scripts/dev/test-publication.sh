#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/publication
records=$(mktemp target/publication/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
for entry in root:canic-root store:ic-wasm-store app:ex-greeting; do
    name=${entry%%:*}
    package=${entry#*:}
    "$cli" artifact build --package "$package" --json > "target/publication/$name.json"
    cp -- "$(jq -r .wasm "target/publication/$name.json")" "target/publication/$name.wasm"
done
"$cli" artifact build --package canic-root --features maintenance-candidate --json > target/publication/root-maintenance.json
cp -- "$(jq -r .wasm target/publication/root-maintenance.json)" target/publication/root-maintenance.wasm
for entry in root-faults:canic-root:fault-injection store-candidate:ic-wasm-store:maintenance-candidate; do
    name=${entry%%:*}
    rest=${entry#*:}
    "$cli" artifact build --package "${rest%%:*}" --features "${rest#*:}" --json > "target/publication/$name.json"
    cp -- "$(jq -r .wasm "target/publication/$name.json")" "target/publication/$name.wasm"
done
if [[ $# == 0 || "${1:-}" == counter_managed:: ]]; then
    "$cli" app build --config apps/examples/counter/canic.toml --canister ex-counter --json > target/publication/counter-client.json
    for variant in v1 v2 fenced; do
        features=()
        case "$variant" in
            v2) features=(--features counter-v2) ;;
            fenced) features=(--features initially-fenced) ;;
        esac
        "$cli" artifact build --package ex-counter "${features[@]}" --output "target/publication/counter-$variant.wasm" --json > "target/publication/counter-$variant.json"
    done
fi
CANIC_CLI="$cli" CANIC_PUBLICATION_ARTIFACTS="$root/target/publication" \
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test publication "$@" -- --nocapture
