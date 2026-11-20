#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
prepare_snapshot() {
    cargo test --locked -p canic-management --features backup --lib
    cargo test --locked -p canic-management-protocol --features backup backup::tests
    mkdir -p target/backup
    "$1" artifact build --package canic-root --features backup,fault-injection --output target/backup/root.wasm --json > target/backup/root.json
    "$1" artifact build --package ex-counter --output target/backup/app.wasm --json > target/backup/app.json
}
if [[ $# == 1 && $1 == snapshot:: ]]; then
    prepare_snapshot canic
    exec cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test root snapshot:: -- --nocapture
fi
mkdir -p target/root
cargo test --locked -p canic-management --lib
records=$(mktemp target/root/build.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected CLI") end' "$records")
if [[ $# == 0 ]]; then
    prepare_snapshot "$cli"
fi
scratch=$(mktemp -d target/root/starter.XXXXXX)
"$cli" app init starter --source "$scratch/project" --canic-source "$root" --json > "$scratch/initialized.json"
"$cli" fleet init mainnet --app starter --network ic --config "$scratch/project/canic.toml" --json > "$scratch/fleet.json"
"$cli" fleet canister list --config "$scratch/project/canic.toml" --json > "$scratch/definitions.json"
"$cli" app build --json --config "$scratch/project/canic.toml" > "$scratch/built.json"
cargo fmt --all --manifest-path "$scratch/project/Cargo.toml" -- --check
cargo clippy --locked --manifest-path "$scratch/project/Cargo.toml" --target wasm32-unknown-unknown -- -D warnings
cp -- "$scratch/project/artifacts/app.wasm" target/root/app.wasm
"$cli" artifact build --json --package canic-greeting-witness --manifest qualification/Cargo.toml --features greeting-v2 > target/root/app-v2.json
cp -- "$(jq -r .wasm target/root/app-v2.json)" target/root/app-v2.wasm
cp -- "$scratch/project/artifacts/root.wasm" target/root/root.wasm
cp -- "$scratch/project/artifacts/store.wasm" target/root/store.wasm
"$cli" artifact build --json --package ic-wasm-store --features maintenance-candidate > target/root/store-maintenance.json
cp -- "$(jq -r .wasm target/root/store-maintenance.json)" target/root/store-maintenance.wasm
"$cli" artifact build --json --package canic-root --features maintenance-candidate > target/root/maintenance.json
cp -- "$(jq -r .wasm target/root/maintenance.json)" target/root/maintenance.wasm
"$cli" artifact build --json --package canic-root --features fault-injection > target/root/faults.json
cp -- "$(jq -r .wasm target/root/faults.json)" target/root/faults.wasm
"$cli" artifact build --package canic-directory-witness --manifest qualification/Cargo.toml --output target/root/directory.wasm --json > target/root/directory.json
# Exact profile-free selections avoid building unrelated historical applications.
# All other filters retain the complete preparation contract.
profile=true
if [[ $# == 1 ]]; then
case "$1" in
    root_controls_one_greeting_application|children::|metrics::|storage_reopen::|predecessor::) profile=false ;;
    *) profile=true ;;
esac
fi
if "$profile"; then
for item in 'canic-profile-manager profile-manager' 'canic-profile-worker profile-worker'; do
    read -r package output <<< "$item"
    "$cli" artifact build --json --package "$package" --manifest qualification/Cargo.toml > "target/root/$output.json"
    cp -- "$(jq -r .wasm "target/root/$output.json")" "target/root/$output.wasm"
done
"$cli" artifact build --json --manifest qualification/Cargo.toml --package canic-profile-worker --features profile-v2 > target/root/profile-worker-v2.json
cp -- "$(jq -r .wasm target/root/profile-worker-v2.json)" target/root/profile-worker-v2.wasm
"$cli" artifact build --json --manifest qualification/Cargo.toml --package canic-profile-manager --features maintenance-candidate > target/root/profile-maintenance.json
cp -- "$(jq -r .wasm target/root/profile-maintenance.json)" target/root/profile-maintenance.wasm
"$cli" artifact build --json --manifest qualification/Cargo.toml --package canic-profile-manager --features fault-injection > target/root/profile-faults.json
cp -- "$(jq -r .wasm target/root/profile-faults.json)" target/root/profile-faults.wasm
fi
CANIC_CLI="$cli" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test root -- --nocapture "$@"
printf 'Generated starter retained at %s/project\n' "$scratch"
