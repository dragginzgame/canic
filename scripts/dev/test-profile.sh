#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/profile
cargo test --locked -p canic-profile-worker --lib
records=$(mktemp target/profile/client.XXXXXX.jsonl)
cargo build --locked -p canic-cli -p canic-profile-client --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
client=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic-profile") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected profile client") end' "$records")
for variant in v1 v2 reject; do
    features=()
    case "$variant" in
        v2) features=(--features profile-v2) ;;
        reject) features=(--features reject-upgrade) ;;
    esac
    "$cli" artifact build --json --package canic-profile-worker "${features[@]}" > "target/profile/$variant.json"
    cp -- "$(jq -r .wasm "target/profile/$variant.json")" "target/profile/$variant.wasm"
done
"$cli" artifact build --json --package canic-profile-entry > target/profile/entry.json
cp -- "$(jq -r .wasm target/profile/entry.json)" target/profile/entry.wasm
CANIC_CLI="$cli" CANIC_PROFILE_CLIENT="$client" cargo test --locked \
    --manifest-path qualification/Cargo.toml -p canic-m1-tests --test profile -- --nocapture
