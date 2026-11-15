#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/blob
perl scripts/dev/blob-inputs.pl > target/blob/inputs.before.json
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --output target/blob/original.wasm --json > target/blob/original.json
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --features maintenance-candidate --output target/blob/candidate.wasm --json > target/blob/candidate.json
CANIC_EFFECT_PROBE_WASM="$root/target/blob/original.wasm" \
    CANIC_BLOB_ARTIFACT_DIRECTORY="$root/target/blob" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification --test blob -- --nocapture
perl scripts/dev/blob-inputs.pl > target/blob/inputs.after.json
cmp target/blob/inputs.before.json target/blob/inputs.after.json
