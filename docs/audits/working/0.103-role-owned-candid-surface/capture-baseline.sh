#!/usr/bin/env bash
set -euo pipefail

readonly EXPECTED_SHA="8cf4723cecd7579cbe3304b980c63b1bc3969d68"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
readonly REPO_ROOT="${1:-$(git rev-parse --show-toplevel)}"
readonly SCRATCH_ROOT="${2:-/tmp/canic-0.103-b1-v0.102.2}"

actual_sha="$(git -C "$REPO_ROOT" rev-parse HEAD)"
if [[ "$actual_sha" != "$EXPECTED_SHA" ]]; then
    echo "baseline capture requires v0.102.2 at $EXPECTED_SHA, found $actual_sha" >&2
    exit 1
fi

if [[ -n "$(git -C "$REPO_ROOT" status --short)" ]]; then
    echo "baseline capture requires a clean v0.102.2 checkout" >&2
    exit 1
fi

mkdir -p "$SCRATCH_ROOT"

build_role() {
    local role="$1"

    cargo run --quiet \
        --manifest-path "$REPO_ROOT/Cargo.toml" \
        -p canic-cli -- \
        build delegation_root_stub "$role" \
        --workspace "$REPO_ROOT" \
        --icp-root "$SCRATCH_ROOT" \
        --config "$REPO_ROOT/canisters/test/delegation_root_stub/canic.toml" \
        --profile fast
}

build_role root
build_role issuer

declare -a profiles=(
    "fleet-subnet-root"
    "managed-auth"
    "fleet-coordinator"
    "wasm-store"
)
declare -a sources=(
    "$SCRATCH_ROOT/.icp/local/canisters/root/root.did"
    "$SCRATCH_ROOT/.icp/local/canisters/issuer/issuer.did"
    "$REPO_ROOT/crates/canic-fleet-coordinator/fleet_coordinator.did"
    "$REPO_ROOT/crates/canic-wasm-store/wasm_store.did"
)

staging="$(mktemp -d /tmp/canic-0.103-candid-capture.XXXXXX)"
trap 'rm -rf "$staging"' EXIT

for index in "${!profiles[@]}"; do
    install -m 0644 "${sources[$index]}" "$staging/${profiles[$index]}.did"
done

printf 'profile\towner_class\tdisposition\tmethod\tmode\tsignature\n' \
    >"$staging/base-methods.tsv"

for profile in "${profiles[@]}"; do
    awk -v profile="$profile" '
        function emit(row, normalized, method, owner, disposition, mode) {
            normalized = row
            gsub(/[[:space:]]+/, " ", normalized)
            sub(/^ /, "", normalized)
            sub(/ .*$/, "", normalized)
            method = normalized
            sub(/ :.*$/, "", method)
            gsub(/^"|"$/, "", method)

            if (method == "icrc10_supported_standards") {
                owner = "external-standard"
                disposition = "external-standard"
            } else if (method !~ /^canic_/) {
                owner = "application-owned"
                disposition = "application-owned"
            } else {
                owner = "canic-owned"
                disposition = "pending-review"
            }

            if (row ~ / composite_query;[[:space:]]*$/) {
                mode = "composite-query"
            } else if (row ~ / query;[[:space:]]*$/) {
                mode = "query"
            } else if (row ~ / oneway;[[:space:]]*$/) {
                mode = "oneway"
            } else {
                mode = "update"
            }

            normalized = row
            gsub(/[[:space:]]+/, " ", normalized)
            sub(/^ /, "", normalized)
            printf "%s\t%s\t%s\t%s\t%s\t%s\n", \
                profile, owner, disposition, method, mode, normalized
        }

        BEGIN { in_service = 0; row = "" }
        /^service[[:space:]]*:/ { in_service = 1; next }
        in_service && /^  ("[^"]+"|[A-Za-z_][A-Za-z0-9_-]*)[[:space:]]*:/ {
            row = $0
            if ($0 ~ /;[[:space:]]*$/) {
                emit(row)
                row = ""
            }
            next
        }
        in_service && row != "" {
            row = row " " $0
            if ($0 ~ /;[[:space:]]*$/) {
                emit(row)
                row = ""
            }
        }
        END {
            if (row != "") {
                print "unterminated Candid service method" > "/dev/stderr"
                exit 1
            }
        }
    ' "$staging/$profile.did" >>"$staging/base-methods.tsv"
done

awk '
    function trim(value) {
        gsub(/[[:space:]]+/, " ", value)
        sub(/^ /, "", value)
        sub(/ $/, "", value)
        return value
    }

    function flush() {
        if (method != "") {
            printf "%s\t%s:%d\t%s\t%s\t%s\t%s\n", method, file, line,
                trim(current_condition), trim(current_attribute), trim(delegate),
                trim(rust_signature)
        }
        method = ""
        delegate = ""
        rust_signature = ""
        collecting_signature = 0
        current_condition = ""
        current_attribute = ""
    }

    FNR == 1 {
        flush()
        file = FILENAME
        sub(/^.*\/crates\/canic\/src\//, "crates/canic/src/", file)
        pending_condition = ""
        pending_condition_line = 0
        pending_attribute = ""
        collecting = 0
    }

    /^[[:space:]]*#\[cfg\(/ {
        pending_condition = trim($0)
        pending_condition_line = FNR
        next
    }

    /^[[:space:]]*#\[\$crate::canic_(query|update)\(/ ||
    /^[[:space:]]*#\[\$crate::__internal::cdk::query/ {
        pending_attribute = trim($0)
        attribute_condition = (FNR == pending_condition_line + 1 ? pending_condition : "")
        pending_condition = ""
        pending_condition_line = 0
        collecting = ($0 !~ /\][[:space:]]*$/)
        next
    }

    collecting {
        pending_attribute = pending_attribute " " trim($0)
        if ($0 ~ /\][[:space:]]*$/) {
            collecting = 0
        }
        next
    }

    match($0, /(async[[:space:]]+)?fn[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*\(/, found) {
        flush()
        method = found[2]
        line = FNR
        current_condition = attribute_condition
        current_attribute = pending_attribute
        rust_signature = trim($0)
        collecting_signature = ($0 !~ /\{/)
        sub(/[[:space:]]*\{.*$/, "", rust_signature)
        pending_condition = ""
        pending_condition_line = 0
        pending_attribute = ""
        attribute_condition = ""
        next
    }

    method != "" && collecting_signature {
        signature_line = trim($0)
        collecting_signature = ($0 !~ /\{/)
        sub(/[[:space:]]*\{.*$/, "", signature_line)
        rust_signature = rust_signature " " signature_line
        next
    }

    method != "" && delegate == "" &&
    ($0 ~ /::api::/ || $0 ~ /(Api|Query|Workflow)::/) {
        delegate = trim($0)
    }

    END { flush() }
' "$REPO_ROOT"/crates/canic/src/macros/endpoints/*.rs \
    >"$staging/source-map.tsv"

awk '
    /ENDPOINT_REPLAY_POLICY_MANIFEST.*= *&\[/ {
        in_manifest = 1
        next
    }
    in_manifest && /^\];/ { exit }
    in_manifest && match($0, /^[[:space:]]+([a-z_]+)\(/, found) {
        helper = found[1]
    }
    in_manifest && match($0, /"(canic_[A-Za-z0-9_]+)"/, found) {
        print found[1] "\t" helper
    }
' "$REPO_ROOT/crates/canic-core/src/replay_policy/endpoint_manifest.rs" \
    >"$staging/replay-map.tsv"

awk '
    match($0, /pub const ([A-Z][A-Z0-9_]+):/, found) {
        constant = found[1]
    }
    constant != "" && match($0, /"(canic_[A-Za-z0-9_]+)"/, found) {
        print found[1] "\t" constant
        constant = ""
    }
' \
    "$REPO_ROOT/crates/canic-core/src/protocol.rs" \
    "$REPO_ROOT/crates/canic/src/protocol.rs" \
    | sort -u >"$staging/protocol-map.tsv"

printf 'method\treferences\n' >"$staging/reference-map.tsv"
awk -F '\t' 'NR > 1 && $2 == "canic-owned" { print $4 }' \
    "$staging/base-methods.tsv" | sort -u |
while IFS= read -r method; do
    constant="$(awk -F '\t' -v method="$method" \
        '$1 == method { print $2; exit }' "$staging/protocol-map.tsv")"
    references="$({
        rg -n -F -w \
            --glob '*.rs' \
            --glob '*.sh' \
            --glob '*.toml' \
            --glob '!target/**' \
            --glob '!.icp/**' \
            --glob '!apps/saltz/**' \
            -- "$method" "$REPO_ROOT" || true
        if [[ -n "$constant" ]]; then
            rg -n -w \
                --glob '*.rs' \
                --glob '*.sh' \
                --glob '*.toml' \
                --glob '!target/**' \
                --glob '!.icp/**' \
                --glob '!apps/saltz/**' \
                -- "$constant" "$REPO_ROOT" || true
        fi
    } | awk -F ':' -v root="$REPO_ROOT/" '
        {
            path = $1
            line = $2
            sub("^" root, "", path)
            if (path ~ /^crates\/canic\/src\/macros\/endpoints\// ||
                path ~ /\/protocol\.rs$/ ||
                path == "crates/canic-core/src/replay_policy/endpoint_manifest.rs") {
                next
            }
            print path ":" line
        }
    ' | sort -u | paste -sd ';' -)"
    printf '%s\t%s\n' "$method" "${references:-none}" \
        >>"$staging/reference-map.tsv"
done

awk -F '\t' '
    FILENAME == ARGV[1] {
        condition = $3
        variant = "generic"
        if (condition == "#[cfg(canic_is_root)]") {
            variant = "root"
        } else if (condition == "#[cfg(not(canic_is_root))]") {
            variant = "nonroot"
        }
        key = $1 SUBSEP variant
        source[key] = $2
        compile_condition[key] = ($3 == "" ? "always" : $3)
        authority[key] = $4
        delegate[key] = $5
        rust_signature[key] = $6
        next
    }
    FILENAME == ARGV[2] { replay[$1] = $2; next }
    FILENAME == ARGV[3] { protocol[$1] = $2; next }
    FILENAME == ARGV[4] && FNR > 1 { references[$1] = $2; next }
    FILENAME == ARGV[5] && FNR == 1 {
        print $0 "\tendpoint_source\tcompile_condition\tauthority_and_payload\tdelegate\treplay_policy\tprotocol_constant\tin_repo_references\trust_signature"
        next
    }
    FILENAME == ARGV[5] {
        if ($2 != "canic-owned") {
            print $0 "\tn/a\tn/a\tn/a\tn/a\tn/a\tn/a\tn/a\tn/a"
            next
        }

        variant = ($1 == "fleet-subnet-root" ? "root" : "nonroot")
        key = $4 SUBSEP variant
        if (!(key in source)) {
            key = $4 SUBSEP "generic"
        }
        replay_policy = replay[$4]
        if (replay_policy == "" && ($5 == "query" || $5 == "composite-query")) {
            replay_policy = "query_read_only(mode-derived)"
        }
        if (replay_policy == "") {
            replay_policy = "missing"
        }
        protocol_constant = protocol[$4]
        if (protocol_constant == "") {
            protocol_constant = "missing"
        }

        print $0 "\t" source[key] "\t" compile_condition[key] "\t" authority[key] \
            "\t" delegate[key] "\t" replay_policy "\t" protocol_constant \
            "\t" references[$4] "\t" rust_signature[key]
    }
' \
    "$staging/source-map.tsv" \
    "$staging/replay-map.tsv" \
    "$staging/protocol-map.tsv" \
    "$staging/reference-map.tsv" \
    "$staging/base-methods.tsv" \
    >"$staging/methods.tsv"

awk -F '\t' '
    NR == 1 { next }
    $2 == "canic-owned" && ($7 == "" || $9 == "" || $10 == "" || $11 == "missing" || $14 == "") {
        print "incomplete Canic method evidence: " $1 "." $4 > "/dev/stderr"
        failed = 1
    }
    END { exit failed }
' "$staging/methods.tsv"

printf 'profile\tsha256\ttotal\tcanic_owned\texternal_standard\tapplication_owned\n' \
    >"$staging/manifest.tsv"

for profile in "${profiles[@]}"; do
    hash="$(sha256sum "$staging/$profile.did" | awk '{ print $1 }')"
    awk -F '\t' -v profile="$profile" -v hash="$hash" '
        NR == 1 || $1 != profile { next }
        { total += 1; owners[$2] += 1 }
        END {
            printf "%s\t%s\t%d\t%d\t%d\t%d\n", profile, hash, total,
                owners["canic-owned"], owners["external-standard"],
                owners["application-owned"]
        }
    ' "$staging/methods.tsv" >>"$staging/manifest.tsv"
done

install -m 0644 "$staging/methods.tsv" "$SCRIPT_DIR/baseline-methods.tsv"
if [[ ! -f "$SCRIPT_DIR/method-register.tsv" ]]; then
    awk -F '\t' '
        function command_target(profile, method) {
            if (profile == "fleet-subnet-root") {
                if (method == "canic_authority_snapshot_prepare") return "RootCommand::PrepareAuthoritySnapshot"
                if (method == "canic_authority_snapshot_resume") return "RootCommand::ResumeAuthoritySnapshot"
                if (method == "canic_fleet_admin") return "RootCommand::{SetCyclesFunding,SetFleetStatus}"
                if (method == "canic_fleet_registry_synchronize") return "RootCommand::SynchronizeRegistry"
                if (method == "canic_fleet_subnet_root_draining_begin") return "RootCommand::RemoveRoot"
                if (method == "canic_fleet_subnet_wasm_store_adopt") return "RootCommand::AdoptStore"
                if (method == "canic_get_or_create_chain_key_delegation_proof") return "RootCommand::GetOrCreateDelegationProof"
                if (method == "canic_icp_refill") return "RootCommand::{PreviewCycleRefill,RefillCycles}"
                if (method == "canic_pool_admin") return "RootCommand::{HandoffPoolCanister,ImportPoolCanister,MaintainPool,RetryPoolRefill,RetryPoolReset}"
                if (method == "canic_prepare_fleet_activation") return "RootCommand::PrepareFleetActivation"
                if (method == "canic_prepare_role_attestation") return "RootCommand::PrepareRoleAttestation"
                if (method == "canic_response_capability_v1") return "RootCommand::RespondCapability"
                if (method == "canic_resume_fleet_activation") return "RootCommand::ResumeFleetActivation"
                if (method == "canic_root_component_allocate") return "RootCommand::ProvisionComponent"
                if (method == "canic_root_component_child_allocate") return "RootCommand::ProvisionChild"
                if (method == "canic_root_component_draining_begin") return "RootCommand::RemoveComponent"
                if (method == "canic_root_component_provisioning_accept") return "RootCommand::ProvisionComponents"
                if (method == "canic_root_component_registry_prepare") return "RootCommand::PrepareComponentRegistry"
                if (method == "canic_root_component_subtree_removal_begin") return "RootCommand::RemoveSubtree"
                if (method == "canic_root_peer_component_allocate") return "RootCommand::ProvisionPeer"
                if (method == "canic_root_store_bootstrap") return "RootCommand::BootstrapStore"
                if (method == "canic_upsert_root_issuer_policy") return "RootCommand::UpsertIssuerPolicy"
                if (method == "canic_upsert_root_issuer_renewal_template") return "RootCommand::UpsertIssuerRenewalTemplate"
                if (method == "canic_wasm_store_admin") return "RootCommand::PublishReleaseSet"
                if (method == "canic_canister_status") return "RootCommand::InspectCanister"
            } else if (profile == "managed-auth") {
                if (method == "canic_component_runtime_directory_prepare") return "CanisterCommand::ConfigureRuntime"
                if (method == "canic_install_active_delegation_proof") return "CanisterCommand::InstallDelegationProof"
                if (method == "canic_prepare_delegated_token") return "CanisterCommand::PrepareDelegatedToken"
                if (method == "canic_response_capability_v1") return "CanisterCommand::RespondCapability"
            } else if (profile == "fleet-coordinator") {
                if (method == "canic_authority_snapshot_prepare") return "CoordinatorCommand::PrepareAuthoritySnapshot"
                if (method == "canic_authority_snapshot_resume") return "CoordinatorCommand::ResumeAuthoritySnapshot"
                if (method == "canic_fleet_component_provisioning_prepare") return "CoordinatorCommand::ProvisionComponents"
                if (method == "canic_fleet_registry_activate") return "CoordinatorCommand::ActivateRegistry"
                if (method == "canic_fleet_registry_root_draining_reservation_prepare") return "CoordinatorCommand::RemoveRoot"
                if (method == "canic_fleet_subnet_root_join") return "CoordinatorCommand::JoinRoot"
            } else if (profile == "wasm-store") {
                if (method == "canic_activate_fleet") return "StoreCommand::ActivateFleet"
                if (method == "canic_prepare_fleet_credential_generation") return "StoreCommand::PrepareFleetCredential"
                if (method == "canic_response_capability_v1") return "StoreCommand::RespondCapability"
                if (method == "canic_sync_state") return "StoreCommand::SynchronizeState"
                if (method == "canic_sync_topology") return "StoreCommand::SynchronizeTopology"
                if (method == "canic_wasm_store_info") return "StoreCommand::InspectTemplate"
                if (method == "canic_wasm_store_prepare") return "StoreCommand::PrepareChunkSet"
                if (method == "canic_wasm_store_prepare_gc") return "StoreCommand::RunGc"
                if (method == "canic_wasm_store_reclaim_deletion_cycles") return "StoreCommand::ReclaimDeletionCycles"
                if (method == "canic_wasm_store_stage_manifest") return "StoreCommand::StageManifest"
            }
            return "unmapped-command"
        }

        function status_target(profile, method) {
            if (profile == "fleet-subnet-root") {
                if (method == "canic_authority_restore_fence_status") return "RootStatusRequest::AuthorityRestore"
                if (method == "canic_bootstrap_status" || method == "canic_ready" ||
                    method == "canic_metadata") return "RootStatusRequest::Overview"
                if (method == "canic_managed_canister_binding") return "RootStatusRequest::Binding"
                if (method == "canic_fleet_subnet_root_authority") return "RootStatusRequest::FleetAuthority"
                if (method == "canic_health") return "RootStatusRequest::Health"
                if (method == "canic_readiness") return "RootStatusRequest::Readiness"
                if (method == "canic_runtime_status") return "RootStatusRequest::Runtime"
                if (method == "canic_cycle_balance") return "RootStatusRequest::CycleBalance"
                if (method == "canic_cycle_topups") return "RootStatusRequest::CycleTopups"
                if (method == "canic_cycle_tracker") return "RootStatusRequest::CycleHistory"
                if (method == "canic_log") return "RootStatusRequest::Logs"
                if (method == "canic_metrics") return "RootStatusRequest::Metrics"
                if (method == "canic_canister_children") return "RootStatusRequest::Children"
                if (method == "canic_fleet_state") return "RootStatusRequest::FleetState"
                if (method == "canic_config") return "RootStatusRequest::Config"
                if (method == "canic_pool_list") return "RootStatusRequest::Pool"
                if (method == "canic_root_component_directory_head") return "RootStatusRequest::ComponentDirectoryHead"
                if (method == "canic_root_component_directory_page") return "RootStatusRequest::ComponentDirectoryPage"
                if (method == "canic_root_component_registry_partition") return "RootStatusRequest::ComponentRegistryPartition"
                if (method == "canic_fleet_subnet_root_canister_summary") return "RootStatusRequest::Inventory"
                if (method == "canic_get_role_attestation") return "RootStatusRequest::RoleAttestation"
                if (method == "canic_root_issuer_renewal_status") return "RootStatusRequest::IssuerRenewal"
                if (method == "canic_wasm_store_overview") return "RootStatusRequest::StoreOverview"
                return "RootStatusRequest::Operation"
            }
            if (profile == "managed-auth") {
                if (method == "canic_bootstrap_status" || method == "canic_ready" ||
                    method == "canic_metadata") return "CanisterStatusRequest::Overview"
                if (method == "canic_managed_canister_binding") return "CanisterStatusRequest::Binding"
                if (method == "canic_health") return "CanisterStatusRequest::Health"
                if (method == "canic_readiness") return "CanisterStatusRequest::Readiness"
                if (method == "canic_runtime_status") return "CanisterStatusRequest::Runtime"
                if (method == "canic_cycle_balance") return "CanisterStatusRequest::CycleBalance"
                if (method == "canic_cycle_topups") return "CanisterStatusRequest::CycleTopups"
                if (method == "canic_cycle_tracker") return "CanisterStatusRequest::CycleHistory"
                if (method == "canic_canister_children") return "CanisterStatusRequest::Children"
                if (method == "canic_log") return "CanisterStatusRequest::Logs"
                if (method == "canic_metrics") return "CanisterStatusRequest::Metrics"
                if (method == "canic_active_delegation_proof_status") return "CanisterStatusRequest::ActiveDelegationProof"
                if (method == "canic_get_delegated_token") return "CanisterStatusRequest::DelegatedToken"
                return "CanisterStatusRequest::Operation"
            }
            if (profile == "fleet-coordinator") {
                if (method == "canic_authority_restore_fence_status") return "CoordinatorStatusRequest::AuthorityRestore"
                if (method == "canic_fleet_component_provisioning_status" ||
                    method == "canic_fleet_registry_root_deletion_execution_status" ||
                    method == "canic_fleet_registry_root_deletion_status") return "CoordinatorStatusRequest::Operation"
                if (method == "canic_fleet_registry") return "CoordinatorStatusRequest::Registry"
                if (method == "canic_fleet_registry_manifest") return "CoordinatorStatusRequest::RegistryManifest"
                if (method == "canic_fleet_registry_root_acknowledgements") return "CoordinatorStatusRequest::RootAcknowledgements"
                if (method == "canic_fleet_registry_version") return "CoordinatorStatusRequest::RegistryVersion"
            }
            if (profile == "wasm-store") {
                if (method == "canic_bootstrap_status" || method == "canic_ready" ||
                    method == "canic_metadata") return "StoreStatusRequest::Overview"
                if (method == "canic_managed_canister_binding") return "StoreStatusRequest::Binding"
                if (method == "canic_cycle_balance") return "StoreStatusRequest::CycleBalance"
                if (method == "canic_cycle_topups") return "StoreStatusRequest::CycleTopups"
                if (method == "canic_cycle_tracker") return "StoreStatusRequest::CycleHistory"
                if (method == "canic_fleet_activation_status") return "StoreStatusRequest::Operation"
                if (method == "canic_wasm_store_catalog") return "StoreStatusRequest::Catalog"
                if (method == "canic_wasm_store_status") return "StoreStatusRequest::Storage"
            }
            return "unmapped-status"
        }

        function executable_caller(reference, path) {
            path = reference
            sub(/:[0-9]+$/, "", path)

            if (path ~ /^crates\/canic-cli\/src\// ||
                path ~ /^crates\/canic-host\/(src|examples)\//) {
                return path !~ /(^|\/)tests?(\/|\.rs$)/
            }
            return path ~ /^crates\/canic-control-plane\/src\/workflow\// ||
                path ~ /^crates\/canic-core\/src\/workflow\// ||
                path ~ /^crates\/canic-core\/src\/ops\/(cascade\.rs|rpc\/)/ ||
                path ~ /^crates\/canic-testing-internal\// ||
                path ~ /^crates\/canic-tests\// ||
                path ~ /^scripts\// ||
                path == "canisters/test/delegation_issuer_stub/src/lib.rs"
        }

        function executable_callers(references, count, parts, idx, result) {
            if (references == "none" || references == "n/a") return "none"
            count = split(references, parts, ";")
            result = ""
            for (idx = 1; idx <= count; idx++) {
                if (executable_caller(parts[idx])) {
                    result = result (result == "" ? "" : ";") parts[idx]
                }
            }
            return result == "" ? "none" : result
        }

        BEGIN { OFS = "\t" }
        NR == 1 {
            print $0, "replacement", "executable_callers"
            next
        }
        NR > 1 && $2 == "canic-owned" &&
        ($5 == "query" || $5 == "composite-query") {
            $3 = ($4 == "canic_wasm_store_bootstrap_debug" ||
                ($1 == "fleet-subnet-root" &&
                    $4 == "canic_managed_canister_binding") ?
                "private-delete" : "role-status-variant")
        }
        NR > 1 && $1 == "wasm-store" && $3 == "pending-review" {
            if ($4 == "canic_wasm_store_chunk" ||
                $4 == "canic_wasm_store_publish_chunk") {
                $3 = "store-data-plane"
            } else if ($4 == "canic_wasm_store_begin_gc" ||
                $4 == "canic_wasm_store_complete_gc") {
                $3 = "private-delete"
            } else {
                $3 = "role-command-variant"
            }
        }
        NR > 1 && $1 == "managed-auth" && $3 == "pending-review" {
            if ($4 == "canic_component_runtime_directory_synchronize" ||
                $4 == "canic_component_runtime_activate") {
                $3 = "private-delete"
            } else {
                $3 = "role-command-variant"
            }
        }
        NR > 1 && $1 == "fleet-coordinator" && $3 == "pending-review" {
            if ($4 == "canic_authority_snapshot_prepare" ||
                $4 == "canic_authority_snapshot_resume" ||
                $4 == "canic_fleet_component_provisioning_prepare" ||
                $4 == "canic_fleet_registry_activate" ||
                $4 == "canic_fleet_registry_root_draining_reservation_prepare" ||
                $4 == "canic_fleet_subnet_root_join") {
                $3 = "role-command-variant"
            } else {
                $3 = "private-delete"
            }
        }
        NR > 1 && $1 == "fleet-subnet-root" && $3 == "pending-review" {
            if ($4 == "canic_canister_status" ||
                $4 == "canic_authority_snapshot_prepare" ||
                $4 == "canic_authority_snapshot_resume" ||
                $4 == "canic_fleet_admin" ||
                $4 == "canic_fleet_registry_synchronize" ||
                $4 == "canic_fleet_subnet_root_draining_begin" ||
                $4 == "canic_fleet_subnet_wasm_store_adopt" ||
                $4 == "canic_get_or_create_chain_key_delegation_proof" ||
                $4 == "canic_icp_refill" ||
                $4 == "canic_pool_admin" ||
                $4 == "canic_prepare_fleet_activation" ||
                $4 == "canic_prepare_role_attestation" ||
                $4 == "canic_response_capability_v1" ||
                $4 == "canic_resume_fleet_activation" ||
                $4 == "canic_root_component_allocate" ||
                $4 == "canic_root_component_child_allocate" ||
                $4 == "canic_root_component_draining_begin" ||
                $4 == "canic_root_component_provisioning_accept" ||
                $4 == "canic_root_component_registry_prepare" ||
                $4 == "canic_root_component_subtree_removal_begin" ||
                $4 == "canic_root_peer_component_allocate" ||
                $4 == "canic_root_store_bootstrap" ||
                $4 == "canic_upsert_root_issuer_policy" ||
                $4 == "canic_upsert_root_issuer_renewal_template" ||
                $4 == "canic_wasm_store_admin") {
                $3 = "role-command-variant"
            } else {
                $3 = "private-delete"
            }
        }
        {
            if ($3 == "private-delete") {
                replacement = "none"
            } else if ($3 == "application-owned" ||
                $3 == "external-standard" || $3 == "store-data-plane") {
                replacement = $4
            } else if ($3 == "role-command-variant") {
                replacement = command_target($1, $4)
            } else if ($3 == "role-status-variant") {
                replacement = status_target($1, $4)
            } else {
                replacement = "unmapped-disposition"
            }
            print $0, replacement, executable_callers($13)
        }
    ' "$staging/methods.tsv" >"$staging/method-register.tsv"
    install -m 0644 "$staging/method-register.tsv" "$SCRIPT_DIR/method-register.tsv"
fi
install -m 0644 "$staging/manifest.tsv" "$SCRIPT_DIR/manifest.tsv"

echo "captured normalized v0.102.2 role Candid baseline in $SCRIPT_DIR"
