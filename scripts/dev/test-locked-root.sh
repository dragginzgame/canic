#!/usr/bin/env bash
# A downstream workspace selects a published Root and builds its two target artifacts.
set -euo pipefail
cli=$(realpath "${1:?pass the built Canic CLI}")
# Use the repository compiler and the caller's existing Cargo compilation cache
# while keeping both final artifacts and the consumer manifest private to this run.
cache=$(cargo metadata --locked --no-deps --format-version 1 | jq -er .build_directory)
export CARGO_BUILD_BUILD_DIR="$cache"
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
cp rust-toolchain.toml "$scratch/"
cd "$scratch"
mkdir src
touch src/lib.rs
cat > Cargo.toml <<'TOML'
[workspace]
[package]
name = "root-consumer"
version = "0.0.0"
edition = "2024"
[dependencies]
canic-root = "=0.253.0"
TOML
cat > canic.toml <<'TOML'
[apps.consumer]
[apps.consumer.canisters.hub]
role = "hub"
parent = "root"
wasm = "artifacts/hub.wasm"
build = { manifest = "Cargo.toml", package = "root-consumer" }
[apps.consumer.fleets.local]
network = { kind = "Local" }
role_receivers = ["hub"]
[apps.consumer.fleets.local.receiver_policy]
# Choose on fresh install. Stable receivers require a Root refresh after upgrade.
storage = "volatile"
root = "root"
roles = ["hub"]
# Replace the local fixture issuer with the installed test issuer before planning.
issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai"
role = "user"
network_id = "1111111111111111111111111111111111111111111111111111111111111111"
application_id = "2222222222222222222222222222222222222222222222222222222222222222"
limits = { default_session_secs = 60, max_session_secs = 1800, max_proof_ttl_ns = 60000000000 }
TOML

# Pin the reviewed archive in the input to Cargo's resolver. Cargo verifies its
# checksum while filling the disposable consumer's transitive dependency graph.
cat > Cargo.lock <<'LOCK'
version = 4

[[package]]
name = "canic-root"
version = "0.253.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b5b368aea2cfb8d78eebfafb62e43098ea812fdb4cc9833c13d5d8f140154061"
LOCK
cargo metadata --quiet --format-version 1 > /dev/null
cp Cargo.lock reviewed.lock
"$cli" app build --canister root
cmp reviewed.lock Cargo.lock
"$cli" --json artifact endpoints artifacts/root.wasm | jq -e '
    [.endpoints[] | select(.mode == "update") | .name] |
    (index("configure_auth") != null and index("prepare_role_key") != null) or error("Root is missing an Auth or role-issuer update")
' > /dev/null
"$cli" artifact inspect artifacts/root.wasm | jq -e '.bytes <= 1900000 or error("Root exceeds the direct-install ceiling")' > /dev/null
artifacts/root.client encode prepare-role-key | jq -e '
    (.method == "prepare_role_key" and (.arguments | length > 0)) or error("Root client failed to encode the role-key command")
' > /dev/null
