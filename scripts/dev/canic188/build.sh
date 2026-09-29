#!/usr/bin/env bash
# Build the frozen incident repair and its separate PocketIC fixture, without deployment.
set -euo pipefail
if [[ $# != 1 ]]; then
    echo "usage: $0 PREPARED_REPAIR_DIRECTORY" >&2
    exit 2
fi
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../../.." && pwd)
repair=$(realpath -- "$1")
case "$repair" in
    "$repo_root"/target/canic188-*) ;;
    *) echo "Repair output must be inside Canic target/canic188-*" >&2; exit 2 ;;
esac
cd -- "$repo_root"
flock -n target/debug/.cargo-lock -c true
bash scripts/ci/run-with-test-scratch.sh cargo test --locked --offline \
    --manifest-path "$repair/control-plane/Cargo.toml" --lib canic188_exact_quiescent_repair

# These are the frozen original role/configuration authorities, not a new release manifest.
export ICP_ENVIRONMENT=ic
export CANIC_ROLE_CONTRACT_VALIDATED=1
export CANIC_INTERNAL_BUILD_CONFIG_PATH="$repair/canic.toml"
export CANIC_INTERNAL_BUILD_ICP_ROOT="$repair"
export CANIC_RELEASE_BUILD_ID=5367c42942e4f1ead6498247b6a3c4bad582848a9b2b49c07632e3722713eb31
export CANIC_PROTOCOL_PROFILE_DIGEST=7ffd9d5035f49468fa0f4e53b7b9a19ccd91f502e9635be2e438eff83d316cf8
unset CANIC_INTERNAL_CANDID_BUILD CANIC_PROTOCOL_BUILD_CONTEXT

for package in root fixture-root; do
    flock -n target/debug/.cargo-lock -c true
    bash scripts/ci/run-with-test-scratch.sh cargo build --locked --offline \
        --manifest-path "$repair/$package/Cargo.toml" \
        --target wasm32-unknown-unknown --profile fast
    cp -- target/wasm32-unknown-unknown/fast/canic_fleet_root.wasm "$repair/$package-template.wasm"
    ic-wasm "$repair/$package-template.wasm" -o "$repair/$package.wasm" shrink
done
gzip -n -9 -c "$repair/root.wasm" > "$repair/root.wasm.gz"
sha256sum "$repair/root.wasm" "$repair/original-root.wasm" > "$repair/artifacts.sha256"
echo "Built incident candidate. Run qualify.sh before retaining a live-review bundle."
