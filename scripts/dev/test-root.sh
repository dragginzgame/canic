#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/root
records=$(mktemp target/root/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
scratch=$(mktemp -d target/root/starter.XXXXXX)
"$cli" app init starter --fleet mainnet --network ic --source "$scratch/project" --canic-source "$root" --json > "$scratch/initialized.json"
"$cli" fleet canister list --config "$scratch/project/canic.json" --json > "$scratch/definitions.json"
"$cli" app build --json --config "$scratch/project/canic.json" > "$scratch/built.json"
cargo fmt --all --manifest-path "$scratch/project/Cargo.toml" -- --check
cargo clippy --locked --manifest-path "$scratch/project/Cargo.toml" --target wasm32-unknown-unknown -- -D warnings
cp -- "$scratch/project/artifacts/app.wasm" target/root/app.wasm
"$cli" artifact build --json --package canic-hello --features greeting-v2 > target/root/app-v2.json
cp -- "$(jq -r .wasm target/root/app-v2.json)" target/root/app-v2.wasm
cp -- "$scratch/project/artifacts/root.wasm" target/root/root.wasm
"$cli" artifact build --json --package canic-root --features maintenance-candidate > target/root/maintenance.json
cp -- "$(jq -r .wasm target/root/maintenance.json)" target/root/maintenance.wasm
CANIC_CLI="$cli" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-m1-tests --test root -- --nocapture
printf 'Generated starter retained at %s/project\n' "$scratch"
