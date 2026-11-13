#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/m1
cargo test --locked -p canic-host --lib
records=$(mktemp target/m1/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
"$cli" --json build > target/m1/v1.json
v1=$(jq -r '.wasm' target/m1/v1.json)
cp "$v1" target/m1/hello-v1.wasm
"$cli" --json build --features greeting-v2 > target/m1/v2.json
v2=$(jq -r '.wasm' target/m1/v2.json)
cp "$v2" target/m1/hello-v2.wasm
CANIC_CLI="$cli" CANIC_HELLO_V1="$root/target/m1/hello-v1.wasm" CANIC_HELLO_V2="$root/target/m1/hello-v2.wasm" \
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-m1-tests --test journey -- --nocapture
