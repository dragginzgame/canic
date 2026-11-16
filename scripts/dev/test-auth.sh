#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/auth
records=$(mktemp target/auth/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
cargo test --locked -p canic --features auth-enrollment auth::enrollment
"$cli" artifact build --package canic-root --features auth --output target/auth/root.wasm --json > target/auth/root.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --output target/auth/receiver.wasm --json > target/auth/receiver.json
CANIC_CLI="$cli" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test auth "$@" -- --nocapture
