#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/store
# The public artifact builder selects Cargo outputs and retains build evidence.
canic artifact build --package canic-wasm-store --json > target/store/original.json
cp -- "$(jq -r .wasm target/store/original.json)" target/store/original.wasm
canic artifact build --package canic-wasm-store --features maintenance-candidate --json > target/store/candidate.json
cp -- "$(jq -r .wasm target/store/candidate.json)" target/store/candidate.wasm
canic artifact build --package canic-hello --json > target/store/app.json
cp -- "$(jq -r .wasm target/store/app.json)" target/store/app.wasm
canic artifact build --package canic-hello --features greeting-v2 --json > target/store/app-v2.json
cp -- "$(jq -r .wasm target/store/app-v2.json)" target/store/app-v2.wasm
canic artifact build --package canic-root --json > target/store/root.json
cp -- "$(jq -r .wasm target/store/root.json)" target/store/root.wasm
canic artifact build --package canic-root --features maintenance-candidate --json > target/store/root-maintenance.json
cp -- "$(jq -r .wasm target/store/root-maintenance.json)" target/store/root-maintenance.wasm
canic artifact build --package canic-root --features fault-injection --json > target/store/root-faults.json
cp -- "$(jq -r .wasm target/store/root-faults.json)" target/store/root-faults.wasm
cargo test --locked -p canic-management-protocol -p canic-wasm-store-protocol -p canic-wasm-store --lib
CANIC_EFFECT_PROBE_WASM="$root/target/store/original.wasm" \
    CANIC_STORE_CANDIDATE="$root/target/store/candidate.wasm" \
    CANIC_STORE_APP_DIRECTORY="$root/target/store" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification --test store -- --nocapture
