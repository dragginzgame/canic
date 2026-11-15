#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/publication
records=$(mktemp target/publication/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
for entry in root:canic-root store:canic-wasm-store app:canic-hello; do
    name=${entry%%:*}
    package=${entry#*:}
    "$cli" artifact build --package "$package" --json > "target/publication/$name.json"
    cp -- "$(jq -r .wasm "target/publication/$name.json")" "target/publication/$name.wasm"
done
CANIC_CLI="$cli" CANIC_PUBLICATION_ARTIFACTS="$root/target/publication" \
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test publication -- --nocapture
