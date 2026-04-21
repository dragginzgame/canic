#!/usr/bin/env bash

require_dfx_ready() {
    local network="${1:-${DFX_NETWORK:-local}}"
    local version_output=""
    local ping_output=""

    if ! command -v dfx >/dev/null 2>&1; then
        echo "dfx is required for this CI script" >&2
        echo "Install it before running CI; do not skip DFX-dependent checks." >&2
        exit 1
    fi

    if ! version_output="$(dfx --version 2>&1)"; then
        echo "dfx is installed but not working" >&2
        echo "$version_output" >&2
        exit 1
    fi

    if ! ping_output="$(dfx ping "$network" 2>&1)"; then
        echo "dfx is not reachable for DFX_NETWORK=$network" >&2
        echo "$ping_output" >&2
        exit 1
    fi
}
