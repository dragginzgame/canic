#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLASSIFIER="$ROOT/scripts/ci/wasm-capability-size-report.jq"

usage() {
    cat <<'EOF'
Usage: scripts/ci/wasm-capability-size-report.sh \
  --wasm <symbol-preserving.wasm> --output <report.json> --role <role> \
  --build-profile <profile> --build-network <local|ic> \
  --producer-identity <immutable-version-or-commit> \
  [--capabilities <comma-separated>] [--metrics-tiers <comma-separated>] \
  [--endpoint-exports <count>]

Creates a machine-readable, disjoint shallow-byte attribution report. Use a
symbol-preserving diagnostic artifact; stripped code[N] entries are reported as
unattributed and are never guessed into a capability.
EOF
}

require_command() {
    local command_name="$1"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required Wasm size-report tool: $command_name" >&2
        exit 2
    fi
}

WASM_PATH=""
OUTPUT_PATH=""
ROLE=""
BUILD_PROFILE=""
BUILD_NETWORK=""
PRODUCER_IDENTITY=""
CAPABILITIES=""
METRICS_TIERS=""
ENDPOINT_EXPORTS="null"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --wasm)
            WASM_PATH="${2:-}"
            shift 2
            ;;
        --output)
            OUTPUT_PATH="${2:-}"
            shift 2
            ;;
        --role)
            ROLE="${2:-}"
            shift 2
            ;;
        --build-profile)
            BUILD_PROFILE="${2:-}"
            shift 2
            ;;
        --build-network)
            BUILD_NETWORK="${2:-}"
            shift 2
            ;;
        --producer-identity)
            PRODUCER_IDENTITY="${2:-}"
            shift 2
            ;;
        --capabilities)
            CAPABILITIES="${2:-}"
            shift 2
            ;;
        --metrics-tiers)
            METRICS_TIERS="${2:-}"
            shift 2
            ;;
        --endpoint-exports)
            ENDPOINT_EXPORTS="${2:-}"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown Wasm size-report argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [[ -z "$WASM_PATH" || -z "$OUTPUT_PATH" || -z "$ROLE" ||
    -z "$BUILD_PROFILE" || -z "$BUILD_NETWORK" || -z "$PRODUCER_IDENTITY" ]]; then
    usage >&2
    exit 2
fi
if [[ ! -f "$WASM_PATH" ]]; then
    echo "Wasm size-report input does not exist: $WASM_PATH" >&2
    exit 2
fi
if [[ "$ENDPOINT_EXPORTS" != "null" && ! "$ENDPOINT_EXPORTS" =~ ^[0-9]+$ ]]; then
    echo "--endpoint-exports must be a non-negative integer" >&2
    exit 2
fi
if [[ "$BUILD_NETWORK" != "local" && "$BUILD_NETWORK" != "ic" ]]; then
    echo "--build-network must be local or ic" >&2
    exit 2
fi

require_command jq
require_command sha256sum
require_command twiggy

OUTPUT_DIR="$(dirname "$OUTPUT_PATH")"
if [[ ! -d "$OUTPUT_DIR" ]]; then
    echo "Wasm size-report output directory does not exist: $OUTPUT_DIR" >&2
    exit 2
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/canic-wasm-capability-size.XXXXXX")"
OUTPUT_TEMP="$(mktemp "$OUTPUT_DIR/.canic-wasm-capability-size.XXXXXX")"
cleanup() {
    rm -rf "$SCRATCH"
    rm -f "$OUTPUT_TEMP"
}
trap cleanup EXIT

ITEMS_PATH="$SCRATCH/twiggy-top.json"
twiggy top -f json "$WASM_PATH" >"$ITEMS_PATH"

ARTIFACT_BYTES="$(wc -c <"$WASM_PATH" | tr -d ' ')"
ARTIFACT_SHA256="$(sha256sum "$WASM_PATH" | awk '{print $1}')"
TWIGGY_VERSION="$(twiggy --version 2>&1 | head -n 1)"

jq -n \
    --slurpfile items "$ITEMS_PATH" \
    --arg file_name "$(basename "$WASM_PATH")" \
    --arg sha256 "$ARTIFACT_SHA256" \
    --argjson artifact_bytes "$ARTIFACT_BYTES" \
    --arg role "$ROLE" \
    --arg build_profile "$BUILD_PROFILE" \
    --arg build_network "$BUILD_NETWORK" \
    --arg producer_identity "$PRODUCER_IDENTITY" \
    --arg capabilities "$CAPABILITIES" \
    --arg metrics_tiers "$METRICS_TIERS" \
    --argjson endpoint_exports "$ENDPOINT_EXPORTS" \
    --arg twiggy_version "$TWIGGY_VERSION" \
    '{
      artifact: {
        file_name: $file_name,
        sha256: $sha256,
        bytes: $artifact_bytes
      },
      context: {
        role: $role,
        build_profile: $build_profile,
        build_network: $build_network,
        producer_identity: $producer_identity,
        role_capabilities: ($capabilities | split(",") | map(select(length > 0)) | sort),
        metrics_tiers: ($metrics_tiers | split(",") | map(select(length > 0)) | sort),
        endpoint_exports: $endpoint_exports
      },
      tool: $twiggy_version,
      items: $items[0]
    }' | jq -f "$CLASSIFIER" >"$OUTPUT_TEMP"

if ! jq -e '.analysis.artifact_bytes_match == true' "$OUTPUT_TEMP" >/dev/null; then
    echo "twiggy shallow-byte total does not match the Wasm artifact size" >&2
    exit 1
fi

mv "$OUTPUT_TEMP" "$OUTPUT_PATH"
echo "Wasm capability size report: $OUTPUT_PATH"
