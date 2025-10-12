#!/bin/bash
set -e

# rustup
rustup update

# cargo
cargo install \
    cargo-audit cargo-bloat cargo-deny cargo-expand cargo-machete \
    cargo-llvm-lines cargo-sort cargo-tarpaulin cargo-sort-derives \
    candid-extractor ic-wasm

# audit
cargo audit

# update
cargo update --verbose

# dfxvm update
# (not update as that can set the default to an older version)
dfxvm self update
