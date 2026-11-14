#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/growth
cargo test --locked -p canic-management-protocol --lib
records=$(mktemp target/growth/clients.XXXXXX.jsonl)
cargo build --locked -p canic-cli -p canic-profile-client --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
client=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic-profile") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected client") end' "$records")
"$cli" artifact build --json --package canic-profile-worker > target/growth/worker.json
cp -- "$(jq -r .wasm target/growth/worker.json)" target/growth/worker.wasm
for variant in manager faults maintenance; do
    features=()
    case "$variant" in
        faults) features=(--features fault-injection) ;;
        maintenance) features=(--features maintenance-candidate) ;;
    esac
    "$cli" artifact build --json --package canic-profile-manager "${features[@]}" > "target/growth/$variant.json"
    cp -- "$(jq -r .wasm "target/growth/$variant.json")" "target/growth/$variant.wasm"
done
CANIC_CLI="$cli" CANIC_PROFILE_CLIENT="$client" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-m1-tests --test growth -- --nocapture
