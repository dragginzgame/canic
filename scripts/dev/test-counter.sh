#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/counter
records=$(mktemp target/counter/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
# Cargo uses the same public filename for both features. Consume the reported
# paths only after both commands finish, as independent fleet builds would.
"$cli" artifact build --json --package ex-counter > target/counter/v1.json &
base_pid=$!
"$cli" artifact build --json --package ex-counter --features incompatible-schema > target/counter/incompatible.json &
incompatible_pid=$!
status=0
wait "$base_pid" || status=$?
wait "$incompatible_pid" || status=$?
if ((status != 0)); then exit "$status"; fi
for variant in v1 incompatible; do
    report="target/counter/$variant.json"
    wasm=$(jq -er .wasm "$report")
    "$cli" artifact inspect "$wasm" --json | jq -e --slurpfile build "$report" '. == $build[0].artifact' >/dev/null
    cp -- "$wasm" "target/counter/$variant.wasm"
done
jq -e -s '.[0].artifact.sha256 != .[1].artifact.sha256' target/counter/{v1,incompatible}.json >/dev/null
"$cli" artifact build --json --package ex-counter --features counter-v2 --output target/counter/v2.wasm > target/counter/v2.json
"$cli" artifact build --json --package ex-counter --features reject-upgrade --output target/counter/reject.wasm > target/counter/reject.json
CANIC_COUNTER_ARTIFACTS="$root/target/counter" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test counter -- --nocapture
