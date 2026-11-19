#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/journey
cargo test --locked -p canic-host --lib
cargo test --locked -p canic-cli --bin canic
records=$(mktemp target/journey/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
"$cli" artifact build --json --package ex-greeting > target/journey/v1.json
v1=$(jq -r '.wasm' target/journey/v1.json)
cp "$v1" target/journey/hello-v1.wasm
"$cli" artifact build --json --package ex-greeting --features greeting-v2 > target/journey/v2.json
v2=$(jq -r '.wasm' target/journey/v2.json)
cp "$v2" target/journey/hello-v2.wasm
CANIC_CLI="$cli" CANIC_HELLO_V1="$root/target/journey/hello-v1.wasm" CANIC_HELLO_V2="$root/target/journey/hello-v2.wasm" \
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test journey -- --nocapture
