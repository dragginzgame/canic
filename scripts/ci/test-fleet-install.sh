#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ENVIRONMENT="${ICP_ENVIRONMENT:-local}"

status_json="$(
    icp --project-root-override "$ROOT" network status -e "$ENVIRONMENT" --json
)"
gateway_url="$(jq -er '.gateway_url | strings | select(length > 0)' <<<"$status_json")"
topology_json="$(
    curl --noproxy '*' --fail --silent --show-error "${gateway_url%/}/_/topology"
)"
application_subnets="$(
    jq -r '
        .subnet_configs
        | to_entries[]
        | select(.value.subnet_kind == "Application")
        | .key
    ' <<<"$topology_json"
)"
application_subnet_count="$(
    awk 'NF { count += 1 } END { print count + 0 }' <<<"$application_subnets"
)"
if [[ "$application_subnet_count" -ne 1 ]]; then
    echo "expected exactly one local Application subnet, found $application_subnet_count" >&2
    exit 1
fi
application_subnet="$(sed -n '1p' <<<"$application_subnets")"

input_path="$(mktemp "${TMPDIR:-/tmp}/canic-test-fleet-input.XXXXXX")"
trap 'rm -f "$input_path"' EXIT
cat >"$input_path" <<EOF
schema_version = 1

[coordinator.subnet]
kind = "explicit"
subnet = "$application_subnet"

[coordinator.creation_funding]
kind = "cycles"
cycles = "20T"

[[fleet_subnet_roots]]
placement_subnet = "$application_subnet"

[fleet_subnet_roots.component_admissions]
app = 1
scaling = 1
test = 1
users = 1

[fleet_subnet_roots.canister_pool]
minimum_size = 1
maximum_size = 10
canister_cycles = "5T"
imports = []

[fleet_subnet_roots.limits]
maximum_component_instances = 4
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 200000000
maximum_group_placements = 16

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "100T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "20T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "20T"
EOF

cd "$ROOT"
CARGO_INCREMENTAL=0 cargo run --locked -q --profile fast -p canic-cli --bin canic -- \
    --environment "$ENVIRONMENT" install --profile fast test test-local \
    --fleet-input "$input_path"
