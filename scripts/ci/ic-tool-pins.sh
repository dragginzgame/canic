#!/usr/bin/env bash
# Product projections of the reviewed common tool matrix; no independent versions.
CANIC_IC_PIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
canic_ic_tool_pin() {
    local tool="$1" host="${2:-}" column="${3:-2}"
    awk -F '\t' -v tool="$tool" -v host="$host" -v column="$column" '
        $1 == tool && (host == "" || $3 == host) {
            if (seen && value != $column) bad=1
            value=$column; seen++
        }
        END { if (!seen || bad) exit 1; print value }
    ' "$CANIC_IC_PIN_ROOT/ci/ic-tools.tsv"
}
CANIC_BINARYEN_VERSION="$(canic_ic_tool_pin wasm-opt)" || return 1
CANIC_IC_WASM_VERSION="$(canic_ic_tool_pin ic-wasm)" || return 1
CANIC_ICP_CLI_VERSION="$(canic_ic_tool_pin icp)" || return 1
export CANIC_BINARYEN_VERSION CANIC_IC_WASM_VERSION CANIC_ICP_CLI_VERSION
