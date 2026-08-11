#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
usage: scripts/dev/skynet-fleet-input.sh <coordinator-subnet> <workload-subnet>...

Generate strict Fleet-install TOML for the Skynet demo. Supply between 8 and 32
distinct workload Subnet principals. The first receives the Authority, the next
seven receive the initial Replicas, and every supplied root is admitted for one
eventual Skynet service member.

The generated policy creates funded mainnet infrastructure. Inspect it before
passing it to `canic install`; this helper never installs or funds anything.
USAGE
}

fail() {
    printf 'Skynet Fleet input generation failed: %s\n' "$1" >&2
    exit 1
}

validate_principal_text() {
    local label="$1"
    local value="$2"

    [[ -n "$value" ]] || fail "$label must not be empty"
    [[ "$value" =~ ^[a-z0-9-]+$ ]] ||
        fail "$label contains characters that are unsafe in generated TOML"
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

if (( $# < 9 || $# > 33 )); then
    usage
    fail "expected one Coordinator and between 8 and 32 workload Subnets"
fi

coordinator_subnet="$1"
shift
workload_subnets=("$@")

validate_principal_text "Coordinator Subnet" "$coordinator_subnet"
for ((index = 0; index < ${#workload_subnets[@]}; index++)); do
    subnet="${workload_subnets[$index]}"
    validate_principal_text "workload Subnet $((index + 1))" "$subnet"
    for ((prior = 0; prior < index; prior++)); do
        if [[ "$subnet" == "${workload_subnets[$prior]}" ]]; then
            fail "workload Subnets $((prior + 1)) and $((index + 1)) are duplicates"
        fi
    done
done

printf '%s\n' \
    '# Generated Skynet Fleet input. Review cycle funding before installation.' \
    'schema_version = 1' \
    '' \
    '[coordinator.subnet]' \
    'kind = "explicit"' \
    "subnet = \"$coordinator_subnet\"" \
    '' \
    '[coordinator.creation_funding]' \
    'kind = "cycles"' \
    'cycles = "3T"'

for ((index = 0; index < ${#workload_subnets[@]}; index++)); do
    subnet="${workload_subnets[$index]}"
    printf '\n%s\n' '[[fleet_subnet_roots]]'
    printf 'placement_subnet = "%s"\n\n' "$subnet"
    printf '%s\n' \
        '[fleet_subnet_roots.component_admissions]' \
        'skynet = 1' \
        '' \
        '[fleet_subnet_roots.component_group_placements]'
    if (( index == 0 )); then
        printf '%s\n' 'skynet_authority = [0]'
    elif (( index < 8 )); then
        printf 'skynet_replicas = [%d]\n' "$((index - 1))"
    fi
    printf '%s\n' \
        '' \
        '[fleet_subnet_roots.canister_pool]' \
        'minimum_size = 1' \
        'maximum_size = 2' \
        'canister_cycles = "2T"' \
        'imports = []' \
        '' \
        '[fleet_subnet_roots.limits]' \
        'maximum_component_instances = 1' \
        'maximum_registry_bytes = 16777216' \
        'maximum_wasm_store_bytes = 100000000' \
        'maximum_group_placements = 1' \
        '' \
        '[fleet_subnet_roots.limits.cycles_funding]' \
        'window_secs = 3600' \
        'maximum_cycles = "100T"' \
        '' \
        '[fleet_subnet_roots.root_creation_funding]' \
        'kind = "cycles"' \
        'cycles = "3T"' \
        '' \
        '[fleet_subnet_roots.wasm_store_creation_funding]' \
        'kind = "cycles"' \
        'cycles = "3T"'
done
