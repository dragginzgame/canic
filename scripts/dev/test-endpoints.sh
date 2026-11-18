#!/usr/bin/env bash
# Focused installed management endpoint contract checks; synthetic cycles only.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/endpoints
records=$(mktemp target/endpoints/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
for variant in root manager faults maintenance worker; do
    features=()
    manifest=qualification/Cargo.toml
    case "$variant" in
        root) package=canic-root; manifest=Cargo.toml ;;
        manager) package=canic-profile-manager ;;
        faults) package=canic-profile-manager; features=(--features fault-injection) ;;
        maintenance) package=canic-profile-manager; features=(--features maintenance-candidate) ;;
        worker) package=canic-profile-worker ;;
    esac
    "$cli" artifact build --json --manifest "$manifest" --package "$package" "${features[@]}" > "target/endpoints/$variant.json"
    cp -- "$(jq -r .wasm "target/endpoints/$variant.json")" "target/endpoints/$variant.wasm"
done
cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test endpoints -- --nocapture
