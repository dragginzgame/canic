#!/usr/bin/env bash
# Materialize the exact CANIC-188 incident artifact inputs inside this checkout.
# This helper performs no network calls, installation, publication or sibling writes.
set -euo pipefail

if [[ $# != 4 ]]; then
    echo "usage: $0 PUBLISHED_CONTROL_PLANE ORIGINAL_ROOT_PACKAGE CONFIG ORIGINAL_ROOT_WASM" >&2
    exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../../.." && pwd)
published=$(realpath -- "$1")
root_package=$(realpath -- "$2")
config=$(realpath -- "$3")
original_wasm=$(realpath -- "$4")

check_hash() {
    local expected=$1 file=$2 actual
    actual=$(sha256sum -- "$file")
    actual=${actual%% *}
    [[ "$actual" == "$expected" ]] || {
        echo "CANIC-188 input hash mismatch: $file" >&2
        exit 1
    }
}

(cd -- "$published" && sha256sum --check --quiet "$script_dir/published-source.sha256")
(cd -- "$root_package" && sha256sum --check "$script_dir/root-package.sha256")
check_hash b34957b58f6243d9f4b31097605906573259706d58331b677dae3df3e8682196 "$config"
check_hash f019558ffac12a214fb68ca68591618d08bdcee6e16858659df24d1ca50f0da6 "$original_wasm"
check_hash e5e4065bf8ddf9e694b47ede061f9e07641389ae2ae670dd0c24ee185300be33 "$script_dir/status.bin"

mkdir -p -- "$repo_root/target"
output=$(mktemp -d "$repo_root/target/canic188-inputs.XXXXXXXX")
cp -a -- "$published" "$output/control-plane"
cp -a -- "$root_package" "$output/root"
cp -- "$config" "$output/canic.toml"
cp -- "$original_wasm" "$output/original-root.wasm"
cp -- "${original_wasm%.wasm}.did" "$output/original-root.did"
cp -- "$script_dir/incident.toml" "$output/incident.toml"
patch --batch --fuzz=0 --directory="$output/control-plane" -p1 < "$script_dir/repair.patch"
cp -- "$script_dir/status.bin" "$output/control-plane/src/ops/canister_pool/capacity_import/canic188-status.bin"
printf '\n[workspace]\n' >> "$output/control-plane/Cargo.toml"
sed -i "s|^canic-control-plane = .*|canic-control-plane = { path = \"$output/control-plane\" }|" "$output/root/Cargo.toml"

# This separate fixture can seed retained records only inside PocketIC.
cp -a -- "$published" "$output/fixture-control-plane"
cp -a -- "$root_package" "$output/fixture-root"
patch --batch --fuzz=0 --directory="$output/fixture-control-plane" -p1 < "$script_dir/fixture.patch"
cp -- "$script_dir/status.bin" "$output/fixture-control-plane/src/ops/canister_pool/capacity_import/canic188-status.bin"
cp -- "$script_dir/fixture-root.rs" "$output/fixture-root/src/lib.rs"
sed -i "s|^canic-control-plane = .*|canic-control-plane = { path = \"$output/fixture-control-plane\" }|" "$output/fixture-root/Cargo.toml"
cp -- "$script_dir/status.bin" "$output/status.bin"

echo "Prepared CANIC-188 inputs: $output"
echo "No installation is authorized or performed by this helper. Follow the qualification record."
