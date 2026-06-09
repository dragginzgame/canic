#!/usr/bin/env bash

_CANIC_REQUIRE_ICP_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_CANIC_REQUIRE_ICP_ROOT_DIR="$(cd "$_CANIC_REQUIRE_ICP_SCRIPT_DIR/../.." && pwd)"
if [ -z "${CANIC_ICP_CLI_VERSION:-}" ] &&
    [ -f "$_CANIC_REQUIRE_ICP_ROOT_DIR/tool-versions.env" ]; then
    # shellcheck source=tool-versions.env
    source "$_CANIC_REQUIRE_ICP_ROOT_DIR/tool-versions.env"
fi

require_icp_tools() {
    local icp_version_output=""
    local ic_wasm_version_output=""
    local required_icp_version="${CANIC_ICP_CLI_VERSION:-}"

    if [ -z "$required_icp_version" ]; then
        echo "missing CANIC_ICP_CLI_VERSION; set it or update tool-versions.env" >&2
        exit 1
    fi

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
