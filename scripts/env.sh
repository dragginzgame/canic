#!/usr/bin/env bash
set -euo pipefail

# Root of the repo
ROOT="${ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)}"

# Network: default to "local" if not set
NETWORK="${NETWORK:-local}"

# Build-time network hint for Rust (used by option_env!("DFX_NETWORK")).
DFX_NETWORK="${DFX_NETWORK:-$NETWORK}"
case "$DFX_NETWORK" in
    local) ;;
    ic|mainnet|staging) DFX_NETWORK="ic" ;;
esac

# Canic config path: default to the repo config.
CANIC_CONFIG_PATH="${CANIC_CONFIG_PATH:-$ROOT/canisters/canic.toml}"

# Export so other commands see them
export ROOT NETWORK DFX_NETWORK CANIC_CONFIG_PATH

# Rust debug output
export RUST_BACKTRACE=1
