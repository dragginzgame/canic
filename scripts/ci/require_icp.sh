#!/usr/bin/env bash

require_icp_tools() {
    local icp_version_output=""
    local ic_wasm_version_output=""
    local required_icp_version="${CANIC_ICP_CLI_VERSION:-0.3.0}"

    if ! command -v icp >/dev/null 2>&1; then
        echo "icp-cli is required for Canic CI" >&2
        echo "Install it with: make install-dev" >&2
        exit 1
    fi

    if ! command -v ic-wasm >/dev/null 2>&1; then
        echo "ic-wasm is required for Canic CI" >&2
        echo "Install it with: make install-dev" >&2
        exit 1
    fi

    if ! icp_version_output="$(icp --version 2>&1)"; then
        echo "icp is installed but not working" >&2
        echo "$icp_version_output" >&2
        exit 1
    fi

    case "$icp_version_output" in
        *" $required_icp_version"|*" $required_icp_version "*)
            ;;
        *)
            echo "unsupported icp-cli version for Canic CI" >&2
            echo "found: $icp_version_output" >&2
            echo "required: icp-cli $required_icp_version" >&2
            echo "Install it with: make install-dev" >&2
            exit 1
            ;;
    esac

    if ! ic_wasm_version_output="$(ic-wasm --version 2>&1)"; then
        echo "ic-wasm is installed but not working" >&2
        echo "$ic_wasm_version_output" >&2
        exit 1
    fi
}
