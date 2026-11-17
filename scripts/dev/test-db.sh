#!/usr/bin/env bash
# Installed application-owned DB through the optional Canic typed mounts.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
out="$root/target/db"
mkdir -p "$out"
cp qualification/Cargo.lock "$out/Cargo.lock"
rustc -Vv > "$out/compiler.txt"
git rev-parse HEAD > "$out/source-head.txt"
tar --exclude=node_modules --exclude=dist -cf "$out/source.tar" Cargo.toml Cargo.lock rust-toolchain.toml crates/canic \
    qualification/Cargo.toml qualification/Cargo.lock qualification/canic-db-witness \
    scripts/dev/test-db.sh
for label in normal fault incompatible; do
    features=()
    if [[ "$label" == fault ]]; then features=(--features fault-injection); fi
    if [[ "$label" == incompatible ]]; then features=(--features incompatible-schema); fi
    cargo tree --manifest-path qualification/Cargo.toml --locked -p canic-db-witness \
        "${features[@]}" --target wasm32-unknown-unknown -e normal,build \
        --format '{p} features=[{f}]' > "$out/$label-features.txt"
    cargo build --manifest-path qualification/Cargo.toml --locked -p canic-db-witness \
        "${features[@]}" --target wasm32-unknown-unknown --release \
        --message-format=json-render-diagnostics > "$out/$label-build.jsonl"
    artifact=$(jq -ers '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic_db_witness") | .filenames[] | select(endswith(".wasm"))] | if length == 1 then .[0] else error("expected one Wasm artifact") end' "$out/$label-build.jsonl")
    cp "$artifact" "$out/$label.wasm"
done
export IC_TESTKIT_SERVER_DIRECTORY="${IC_TESTKIT_SERVER_DIRECTORY:-$root/target/qualification/server/prepared}"
CANIC_DB_WASM="$out/normal.wasm" CANIC_DB_FAULT_WASM="$out/fault.wasm" CANIC_DB_INCOMPATIBLE_WASM="$out/incompatible.wasm" \
    cargo test --manifest-path qualification/Cargo.toml --locked -p canic-db-witness \
    --test installed -- --nocapture 2>&1 | tee "$out/run.log"
