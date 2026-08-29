#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLASSIFIER="$ROOT/scripts/ci/wasm-capability-size-report.jq"
if ! command -v jq >/dev/null 2>&1; then
    echo "missing required Wasm capability size test tool: jq" >&2
    exit 2
fi
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-wasm-capability-size-test.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT

jq -n '{
  artifact: {file_name: "diagnostic.wasm", sha256: "fixture", bytes: 400},
  context: {
    role: "project_instance",
    build_profile: "debug",
    build_network: "ic",
    producer_identity: "fixture-a",
    role_capabilities: ["Runtime", "ChildProvisioning"],
    metrics_tiers: ["Core", "Runtime", "Security"],
    endpoint_exports: 273
  },
  tool: "twiggy fixture",
  items: [
    {name: "canic_core::ops::auth::verify", shallow_size: 100},
    {name: "canic_metrics_core::encode", shallow_size: 80},
    {name: "canic_control_plane::child::create", shallow_size: 70},
    {name: "canic_core::workflow::status", shallow_size: 50},
    {name: "project_instance::endpoint", shallow_size: 40},
    {name: "code[12]", shallow_size: 30},
    {name: "data[0]", shallow_size: 15},
    {name: "type[1]: (i32) -> nil", shallow_size: 5},
    {name: "export \"canister_update endpoint\"", shallow_size: 10}
  ]
}' | jq -f "$CLASSIFIER" >"$FIXTURE/partial.json"

jq -e '
  .schema == "canic.wasm_capability_size.v1"
  and .analysis.artifact_bytes_match == true
  and .analysis.symbol_attribution == "partial"
  and .analysis.named_code_bytes == 340
  and .analysis.unattributed_code_bytes == 30
  and ([.categories[] | {key: .category, value: .shallow_bytes}] | from_entries) == {
    authentication_and_admission: 100,
    metrics: 80,
    child_provisioning: 70,
    canic_runtime: 50,
    application_and_upstream: 40,
    unattributed_code: 30,
    wasm_structural_and_abi: 30
  }
' "$FIXTURE/partial.json" >/dev/null

jq -n '{
  artifact: {file_name: "stripped.wasm", sha256: "fixture", bytes: 80},
  context: {
    role: "project_instance",
    build_profile: "release",
    build_network: "ic",
    producer_identity: "fixture-b",
    role_capabilities: [],
    metrics_tiers: [],
    endpoint_exports: null
  },
  tool: "twiggy fixture",
  items: [
    {name: "code[7]", shallow_size: 60},
    {name: "data[0]", shallow_size: 20}
  ]
}' | jq -f "$CLASSIFIER" >"$FIXTURE/stripped.json"

jq -e '
  .analysis.symbol_attribution == "unavailable"
  and .analysis.named_code_bytes == 0
  and .analysis.unattributed_code_bytes == 60
  and (.categories[] | select(.category == "canic_runtime") | .shallow_bytes) == 0
' "$FIXTURE/stripped.json" >/dev/null

echo "Wasm capability size report tests passed"
