#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/blob
records=$(mktemp target/blob/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
for entry in root:canic-root store:ic-wasm-store; do
    name=${entry%%:*}
    features=()
    if [[ "$name" == root ]]; then features=(--features blob,fault-injection); fi
    "$cli" artifact build --package "${entry#*:}" "${features[@]}" --output "target/blob/$name.wasm" --json > "target/blob/$name.json"
done
perl scripts/dev/blob-inputs.pl > target/blob/inputs.before.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --output target/blob/original.wasm --json > target/blob/original.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --features maintenance-candidate --output target/blob/candidate.wasm --json > target/blob/candidate.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --features profile --output target/blob/composed.wasm --json > target/blob/composed.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --features profile,maintenance-candidate --output target/blob/composed-candidate.wasm --json > target/blob/composed-candidate.json
CANIC_CLI="$cli" CANIC_EFFECT_PROBE_WASM="$root/target/blob/original.wasm" \
    CANIC_BLOB_ARTIFACT_DIRECTORY="$root/target/blob" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification --features profile-state --test blob "$@" -- --nocapture
perl scripts/dev/blob-inputs.pl > target/blob/inputs.after.json
cmp target/blob/inputs.before.json target/blob/inputs.after.json
