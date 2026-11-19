#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
browser=false
if [[ "${1:-}" == --profile-browser ]]; then
    browser=true
    shift
    : "${CANIC_AUTH_BROWSER_EXECUTABLE:?Select an explicitly prepared Chromium executable}"
fi
mkdir -p target/auth
records=$(mktemp target/auth/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
"$cli" artifact build --package canic-root --features auth --output target/auth/root.wasm --json > target/auth/root.json
if [[ "$browser" == true ]]; then
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --features profile --output target/auth/profile.wasm --json > target/auth/profile.json
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features profile-state --test auth_browser "$@" -- --ignored --nocapture
    exit
fi
cargo test --locked -p canic --features auth-enrollment auth::
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --output target/auth/receiver.wasm --json > target/auth/receiver.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --features stable --output target/auth/stable.wasm --json > target/auth/stable.json
"$cli" artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --features damaged-checkpoint --output target/auth/damaged.wasm --json > target/auth/damaged.json
CANIC_CLI="$cli" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test auth "$@" -- --nocapture
