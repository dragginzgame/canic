//! Module: dto::fleet_subnet_root
//!
//! Responsibility: carry protected Fleet Subnet Root authority and controller lifecycle DTOs.
//! Does not own: validation, persistence, topology compilation, or lifecycle effects.
//! Boundary: lifecycle adapters pass init/command authority to workflow and return passive data.

use crate::{
    dto::fleet_registry::{FleetRegistryVersion, FleetSubnetRootStatus},
    ids::{
        ComponentTopologyDigest, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
        FleetSubnetWasmStoreActivationAuthority, FleetSubnetWasmStoreAuthority, SubnetId,
    },
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Execution balance retained while a removed root completes its deletion handoff.
pub const FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES: u128 = 300_000_000_000;

/// Margin for call-context refunds that become visible after a cycle transfer returns.
pub const FLEET_SUBNET_ROOT_DELETION_CALL_REFUND_HEADROOM_CYCLES: u128 = 150_000_000_000;

///
/// FleetSubnetRootAuthority
///
/// Exact immutable root binding, initial release set, and installed module identity.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetSubnetRootAuthority {
    pub binding: FleetSubnetRootBinding,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    pub expected_module_hash: [u8; 32],
    pub wasm_store_authority: FleetSubnetWasmStoreAuthority,
}

///
/// FleetSubnetWasmStoreInitArgs
///
/// Fresh-install operation identity plus one complete sibling Store authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetSubnetWasmStoreInitArgs {
    pub authority: FleetSubnetWasmStoreAuthority,
    pub install_id: [u8; 32],
}

/// Request the one planned sibling Store controller handoff during root preparation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetWasmStoreAdoptionRequest {
    pub operation_id: [u8; 32],
    pub authority: FleetSubnetWasmStoreAuthority,
}

/// Terminal root-observed receipt for one sibling Store controller handoff.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetWasmStoreAdoptionResponse {
    pub operation_id: [u8; 32],
    pub authority: FleetSubnetWasmStoreAuthority,
    pub controllers: Vec<Principal>,
    pub adopted_at_ns: u64,
}

///
/// FleetSubnetRootCanisterSummary
///
/// Compact live inventory bound to one root's exact active Fleet Registry mirror.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootCanisterSummary {
    pub fleet_registry: FleetRegistryVersion,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub status: FleetSubnetRootStatus,
    pub infrastructure_canisters: u32,
    pub component_canisters: u32,
    pub pooled_canisters: u32,
    pub total_canisters: u32,
}

///
/// FleetSubnetRootDrainingRequest
///
/// Controller command fencing new top-level Component allocation under exact active authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingRequest {
    pub operation_id: [u8; 32],
    pub expected_registry: FleetRegistryVersion,
}

///
/// FleetSubnetRootDrainingStatusRequest
///
/// Read-only lookup key for one durable root-draining fence.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingStatusRequest {
    pub operation_id: [u8; 32],
}

///
/// FleetSubnetRootDrainingResponse
///
/// Durable root-local admission cutoff and exact active authority frozen at that boundary.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub active_registry: FleetRegistryVersion,
    pub reservation_hash: [u8; 32],
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub known_created_component_canisters: u32,
    pub root_registry_encoded_bytes: u64,
    pub started_at_ns: u64,
}

///
/// FleetSubnetRootFinalInventoryRequest
///
/// Controller command freezing one exact terminal root-local inventory.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootFinalInventoryRequest {
    pub operation_id: [u8; 32],
    pub expected_registry: FleetRegistryVersion,
}

///
/// FleetSubnetRootFinalInventoryStatusRequest
///
/// Read-only lookup key for one durable terminal root-local inventory.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootFinalInventoryStatusRequest {
    pub operation_id: [u8; 32],
}

///
/// FleetSubnetRootRemovalRequest
///
/// Controller command revalidating terminal Store authority before logical root removal.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRemovalRequest {
    pub operation_id: [u8; 32],
    pub expected_registry: FleetRegistryVersion,
}

/// Read-only lookup key for one durable logical root-removal publication.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRemovalStatusRequest {
    pub operation_id: [u8; 32],
}

///
/// FleetSubnetRootStoreReclamationRequest
///
/// Controller command reclaiming the retained Store after exact logical root removal.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreReclamationRequest {
    pub operation_id: [u8; 32],
    pub expected_final_inventory_hash: [u8; 32],
}

/// Read-only lookup key for one durable root Store-reclamation receipt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreReclamationStatusRequest {
    pub operation_id: [u8; 32],
}

///
/// FleetSubnetRootStoreReclamationResponse
///
/// Durable proof that the logically removed root's retained Store completed exact GC.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreReclamationResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub final_inventory_hash: [u8; 32],
    pub reclaimed_store_bytes: u64,
    pub reclaimed_catalog_entries: u32,
    pub reclaimed_template_count: u32,
    pub reclaimed_release_count: u32,
    pub gc_prepared_at_secs: u64,
    pub gc_started_at_secs: u64,
    pub gc_completed_at_secs: u64,
    pub gc_runs_completed: u32,
    pub completed_at_ns: u64,
    pub reclamation_hash: [u8; 32],
}

/// Controller command finalizing the reclaimed Store's root-local binding.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreBindingFinalizationRequest {
    pub operation_id: [u8; 32],
    pub expected_reclamation_hash: [u8; 32],
}

/// Read-only lookup key for one durable Store-binding finalization receipt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreBindingFinalizationStatusRequest {
    pub operation_id: [u8; 32],
}

/// Durable proof that the reclaimed Store no longer occupies a publication binding slot.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreBindingFinalizationResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub final_inventory_hash: [u8; 32],
    pub reclamation_hash: [u8; 32],
    pub source_generation: u64,
    pub finalized_generation: u64,
    pub finalized_at_secs: u64,
    pub completed_at_ns: u64,
    pub finalization_hash: [u8; 32],
}

/// Controller command physically deleting the reclaimed and unbound Store.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreDeletionRequest {
    pub operation_id: [u8; 32],
    pub expected_binding_finalization_hash: [u8; 32],
}

/// Read-only lookup key for one durable Store-deletion receipt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreDeletionStatusRequest {
    pub operation_id: [u8; 32],
}

/// Durable proof that the root's reclaimed and unbound Store is physically absent.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootStoreDeletionResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub binding_finalization_hash: [u8; 32],
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
    pub prepared_at_ns: u64,
    pub observed_absent_at_ns: u64,
    pub completed_at_ns: u64,
    pub deletion_hash: [u8; 32],
}

/// Controller command preparing a removed root for external physical deletion.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionPreparationRequest {
    pub operation_id: [u8; 32],
    pub expected_store_deletion_hash: [u8; 32],
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
}

/// Read-only lookup key for the root's durable external-deletion readiness receipt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionPreparationStatusRequest {
    pub operation_id: [u8; 32],
}

/// Durable proof that a removed root returned excess cycles and is ready for its executor.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionPreparationResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub coordinator: Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub retained_cycles_target: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
    pub coordinator_intent_hash: [u8; 32],
    pub coordinator_readiness_hash: [u8; 32],
    pub prepared_at_ns: u64,
    pub completed_at_ns: u64,
}

///
/// FleetSubnetRootFinalInventoryResponse
///
/// Exact terminal Component history and retained write-fenced Store authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootFinalInventoryResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub registry: FleetRegistryVersion,
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub removed_component_instances: u32,
    pub terminal_component_history_hash: [u8; 32],
    pub root_registry_encoded_bytes: u64,
    pub wasm_store: Principal,
    pub wasm_store_catalog_hash: [u8; 32],
    pub wasm_store_catalog_entries: u32,
    pub wasm_store_occupied_bytes: u64,
    pub wasm_store_template_count: u32,
    pub wasm_store_release_count: u32,
    pub wasm_store_gc_prepared_at_secs: u64,
    pub finalized_at_ns: u64,
    pub inventory_hash: [u8; 32],
}

///
/// FleetSubnetRootInitArgs
///
/// Fresh-install authority plus the reinstall-local activation operation identity.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetSubnetRootInitArgs {
    pub authority: FleetSubnetRootAuthority,
    pub install_id: [u8; 32],
    pub wasm_store_activation: FleetSubnetWasmStoreActivationAuthority,
    /// Existing prepaid empty Canisters the root must validate, reset, and adopt.
    pub canister_pool_imports: Vec<Principal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority,
    };

    #[test]
    fn canister_summary_and_root_lifecycle_contracts_round_trip_through_candid() {
        let summary = canister_summary();
        let candid = candid::encode_one(&summary).expect("encode Canister summary");
        let decoded: FleetSubnetRootCanisterSummary =
            candid::decode_one(&candid).expect("decode Canister summary");

        assert_eq!(decoded, summary);

        let draining = draining_response(&summary);
        let request = FleetSubnetRootDrainingRequest {
            operation_id: draining.operation_id,
            expected_registry: draining.active_registry.clone(),
        };
        let status = FleetSubnetRootDrainingStatusRequest {
            operation_id: draining.operation_id,
        };
        let request_bytes = candid::encode_one(&request).expect("encode root draining request");
        let status_bytes = candid::encode_one(status).expect("encode root draining status");
        let response_bytes = candid::encode_one(&draining).expect("encode root draining response");
        assert_eq!(
            candid::decode_one::<FleetSubnetRootDrainingRequest>(&request_bytes)
                .expect("decode root draining request"),
            request
        );
        assert_eq!(
            candid::decode_one::<FleetSubnetRootDrainingStatusRequest>(&status_bytes)
                .expect("decode root draining status"),
            status
        );
        assert_eq!(
            candid::decode_one::<FleetSubnetRootDrainingResponse>(&response_bytes)
                .expect("decode root draining response"),
            draining
        );

        let inventory = final_inventory_response(&draining);
        let inventory_request = FleetSubnetRootFinalInventoryRequest {
            operation_id: inventory.operation_id,
            expected_registry: inventory.registry.clone(),
        };
        let inventory_status = FleetSubnetRootFinalInventoryStatusRequest {
            operation_id: inventory.operation_id,
        };
        let request_bytes =
            candid::encode_one(&inventory_request).expect("encode root inventory request");
        let status_bytes =
            candid::encode_one(inventory_status).expect("encode root inventory status");
        let response_bytes =
            candid::encode_one(&inventory).expect("encode root inventory response");
        assert_eq!(
            candid::decode_one::<FleetSubnetRootFinalInventoryRequest>(&request_bytes)
                .expect("decode root inventory request"),
            inventory_request
        );
        assert_eq!(
            candid::decode_one::<FleetSubnetRootFinalInventoryStatusRequest>(&status_bytes)
                .expect("decode root inventory status"),
            inventory_status
        );
        assert_eq!(
            candid::decode_one::<FleetSubnetRootFinalInventoryResponse>(&response_bytes)
                .expect("decode root inventory response"),
            inventory
        );
    }

    #[test]
    fn draining_publication_contracts_round_trip_through_candid() {
        let draining = draining_response(&canister_summary());
        let publication = crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationRequest {
            expected_registry: draining.active_registry.clone(),
            root_draining: draining.clone(),
        };
        let publication_response =
            crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse {
                root_draining: draining,
                previous_version: publication.expected_registry.clone(),
                version: FleetRegistryVersion {
                    authority: publication.expected_registry.authority.clone(),
                    revision: publication.expected_registry.revision + 1,
                    content_hash: [19; 32],
                },
            };
        let publication_bytes =
            candid::encode_one(&publication).expect("encode root draining publication");
        let publication_response_bytes = candid::encode_one(&publication_response)
            .expect("encode root draining publication response");
        assert_eq!(
            candid::decode_one::<
                crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationRequest,
            >(&publication_bytes)
            .expect("decode root draining publication"),
            publication
        );
        assert_eq!(
            candid::decode_one::<
                crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse,
            >(&publication_response_bytes)
            .expect("decode root draining publication response"),
            publication_response
        );

        let final_inventory = final_inventory_response(&publication_response.root_draining);
        let removal_request = FleetSubnetRootRemovalRequest {
            operation_id: final_inventory.operation_id,
            expected_registry: publication_response.version.clone(),
        };
        let removal_status = FleetSubnetRootRemovalStatusRequest {
            operation_id: final_inventory.operation_id,
        };
        let coordinator_request =
            crate::dto::fleet_registry::FleetSubnetRootRemovalPublicationRequest {
                expected_registry: publication_response.version.clone(),
                final_inventory: final_inventory.clone(),
            };
        let coordinator_response =
            crate::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse {
                final_inventory,
                previous_version: publication_response.version.clone(),
                version: FleetRegistryVersion {
                    authority: publication_response.version.authority.clone(),
                    revision: publication_response.version.revision + 1,
                    content_hash: [29; 32],
                },
            };
        assert_candid_round_trip(&removal_request);
        assert_candid_round_trip(&removal_status);
        assert_candid_round_trip(&coordinator_request);
        assert_candid_round_trip(&coordinator_response);

        let reclamation_request = FleetSubnetRootStoreReclamationRequest {
            operation_id: coordinator_response.final_inventory.operation_id,
            expected_final_inventory_hash: coordinator_response.final_inventory.inventory_hash,
        };
        let reclamation_status = FleetSubnetRootStoreReclamationStatusRequest {
            operation_id: reclamation_request.operation_id,
        };
        let reclamation_response = FleetSubnetRootStoreReclamationResponse {
            operation_id: reclamation_request.operation_id,
            fleet_subnet_root: coordinator_response.final_inventory.fleet_subnet_root,
            wasm_store: coordinator_response.final_inventory.wasm_store,
            final_inventory_hash: reclamation_request.expected_final_inventory_hash,
            reclaimed_store_bytes: coordinator_response
                .final_inventory
                .wasm_store_occupied_bytes,
            reclaimed_catalog_entries: coordinator_response
                .final_inventory
                .wasm_store_catalog_entries,
            reclaimed_template_count: coordinator_response
                .final_inventory
                .wasm_store_template_count,
            reclaimed_release_count: coordinator_response
                .final_inventory
                .wasm_store_release_count,
            gc_prepared_at_secs: coordinator_response
                .final_inventory
                .wasm_store_gc_prepared_at_secs,
            gc_started_at_secs: 30,
            gc_completed_at_secs: 31,
            gc_runs_completed: 1,
            completed_at_ns: 32,
            reclamation_hash: [33; 32],
        };
        assert_candid_round_trip(&reclamation_request);
        assert_candid_round_trip(&reclamation_status);
        assert_candid_round_trip(&reclamation_response);

        let finalization =
            assert_store_binding_finalization_contract_round_trip(&reclamation_response);
        assert_store_deletion_contract_round_trip(&finalization);
    }

    fn assert_store_binding_finalization_contract_round_trip(
        reclamation: &FleetSubnetRootStoreReclamationResponse,
    ) -> FleetSubnetRootStoreBindingFinalizationResponse {
        let request = FleetSubnetRootStoreBindingFinalizationRequest {
            operation_id: reclamation.operation_id,
            expected_reclamation_hash: reclamation.reclamation_hash,
        };
        let status = FleetSubnetRootStoreBindingFinalizationStatusRequest {
            operation_id: request.operation_id,
        };
        let response = FleetSubnetRootStoreBindingFinalizationResponse {
            operation_id: request.operation_id,
            fleet_subnet_root: reclamation.fleet_subnet_root,
            wasm_store: reclamation.wasm_store,
            final_inventory_hash: reclamation.final_inventory_hash,
            reclamation_hash: request.expected_reclamation_hash,
            source_generation: 4,
            finalized_generation: 7,
            finalized_at_secs: 34,
            completed_at_ns: 35,
            finalization_hash: [36; 32],
        };
        assert_candid_round_trip(&request);
        assert_candid_round_trip(&status);
        assert_candid_round_trip(&response);
        response
    }

    fn assert_store_deletion_contract_round_trip(
        finalization: &FleetSubnetRootStoreBindingFinalizationResponse,
    ) {
        let request = FleetSubnetRootStoreDeletionRequest {
            operation_id: finalization.operation_id,
            expected_binding_finalization_hash: finalization.finalization_hash,
        };
        let status = FleetSubnetRootStoreDeletionStatusRequest {
            operation_id: request.operation_id,
        };
        let response = FleetSubnetRootStoreDeletionResponse {
            operation_id: request.operation_id,
            fleet_subnet_root: finalization.fleet_subnet_root,
            wasm_store: finalization.wasm_store,
            binding_finalization_hash: finalization.finalization_hash,
            observed_module_hash: [37; 32],
            observed_controllers: vec![finalization.fleet_subnet_root],
            observed_cycles_before_reclamation: 500,
            retained_cycles_target: 100,
            observed_cycles_after_reclamation: 90,
            cycles_reclaimed_at_ns: 38,
            prepared_at_ns: 38,
            observed_absent_at_ns: 39,
            completed_at_ns: 40,
            deletion_hash: [41; 32],
        };
        assert_candid_round_trip(&request);
        assert_candid_round_trip(&status);
        assert_candid_round_trip(&response);
        assert_root_deletion_handoff_contract_round_trip(
            &response,
            finalization.final_inventory_hash,
        );
    }

    fn assert_root_deletion_handoff_contract_round_trip(
        store_deletion: &FleetSubnetRootStoreDeletionResponse,
        final_inventory_hash: [u8; 32],
    ) {
        use crate::dto::fleet_registry::{
            FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionReadinessResponse,
        };

        let preparation_request = FleetSubnetRootDeletionPreparationRequest {
            operation_id: store_deletion.operation_id,
            expected_store_deletion_hash: store_deletion.deletion_hash,
            retained_cycles_target: 100_000_000_001,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
        };
        let preparation_status = FleetSubnetRootDeletionPreparationStatusRequest {
            operation_id: store_deletion.operation_id,
        };
        let intent_request = FleetSubnetRootDeletionReadinessIntentRequest {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
            final_inventory_hash,
            store_deletion_hash: store_deletion.deletion_hash,
            observed_cycles_before_reclamation: 500_000_000_000,
            retained_cycles_target: 100_000_000_001,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
            prepared_at_ns: 42,
        };
        let intent = FleetSubnetRootDeletionReadinessIntentResponse {
            request: intent_request.clone(),
            coordinator: Principal::from_slice(&[43; 29]),
            recorded_at_ns: 44,
            intent_hash: [45; 32],
        };
        let readiness_request = FleetSubnetRootDeletionReadinessRequest {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
            expected_intent_hash: intent.intent_hash,
            observed_cycles_after_reclamation: 90_000_000_000,
            cycles_reclaimed_at_ns: 46,
        };
        let readiness = FleetSubnetRootDeletionReadinessResponse {
            request: readiness_request.clone(),
            coordinator: intent.coordinator,
            final_inventory_hash,
            store_deletion_hash: store_deletion.deletion_hash,
            observed_cycles_before_reclamation: 500_000_000_000,
            retained_cycles_target: 100_000_000_001,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
            prepared_at_ns: intent.request.prepared_at_ns,
            recorded_at_ns: 47,
            readiness_hash: [48; 32],
        };
        assert_root_deletion_execution_contract_round_trip(store_deletion, &readiness);

        let preparation = FleetSubnetRootDeletionPreparationResponse {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
            coordinator: intent.coordinator,
            final_inventory_hash,
            store_deletion_hash: store_deletion.deletion_hash,
            observed_cycles_before_reclamation: 500_000_000_000,
            retained_cycles_target: 100_000_000_001,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
            observed_cycles_after_reclamation: 90_000_000_000,
            cycles_reclaimed_at_ns: readiness_request.cycles_reclaimed_at_ns,
            coordinator_intent_hash: intent.intent_hash,
            coordinator_readiness_hash: readiness.readiness_hash,
            prepared_at_ns: intent.request.prepared_at_ns,
            completed_at_ns: 56,
        };

        assert_candid_round_trip(&preparation_request);
        assert_candid_round_trip(&preparation_status);
        assert_candid_round_trip(&preparation);
        assert_candid_round_trip(&intent_request);
        assert_candid_round_trip(&intent);
        assert_candid_round_trip(&readiness_request);
        assert_candid_round_trip(&readiness);
    }

    fn assert_root_deletion_execution_contract_round_trip(
        store_deletion: &FleetSubnetRootStoreDeletionResponse,
        readiness: &crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse,
    ) {
        use crate::dto::fleet_registry::{
            FleetSubnetRootDeletionCompletionRequest, FleetSubnetRootDeletionExecutionRequest,
            FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDeletionStatusRequest,
        };

        let executor = Principal::from_slice(&[49; 29]);
        let execution_request = FleetSubnetRootDeletionExecutionRequest {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
            expected_readiness_hash: readiness.readiness_hash,
            observed_module_hash: [50; 32],
            observed_controllers: vec![executor],
            observed_cycles_after_reclamation: 90_000_000_000,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
        };
        let execution = FleetSubnetRootDeletionExecutionResponse {
            request: execution_request.clone(),
            executor,
            prepared_at_ns: 51,
            execution_hash: [52; 32],
        };
        let completion_request = FleetSubnetRootDeletionCompletionRequest {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
            expected_execution_hash: execution.execution_hash,
            observed_absent_at_ns: 53,
        };
        let status = FleetSubnetRootDeletionStatusRequest {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
        };
        let deletion = FleetSubnetRootDeletionResponse {
            operation_id: store_deletion.operation_id,
            fleet_subnet_root: store_deletion.fleet_subnet_root,
            coordinator: readiness.coordinator,
            executor,
            readiness_hash: readiness.readiness_hash,
            execution_hash: execution.execution_hash,
            observed_module_hash: execution_request.observed_module_hash,
            observed_controllers: execution_request.observed_controllers.clone(),
            observed_cycles_after_reclamation: 90_000_000_000,
            observed_absent_at_ns: completion_request.observed_absent_at_ns,
            completed_at_ns: 54,
            deletion_hash: [55; 32],
        };
        assert_candid_round_trip(&execution_request);
        assert_candid_round_trip(&execution);
        assert_candid_round_trip(&completion_request);
        assert_candid_round_trip(&status);
        assert_candid_round_trip(&deletion);
    }

    fn assert_candid_round_trip<T>(value: &T)
    where
        T: CandidType + for<'de> candid::Deserialize<'de> + Eq + std::fmt::Debug,
    {
        let bytes = candid::encode_one(value).expect("encode Candid contract");
        assert_eq!(
            &candid::decode_one::<T>(&bytes).expect("decode Candid contract"),
            value,
        );
    }

    fn canister_summary() -> FleetSubnetRootCanisterSummary {
        FleetSubnetRootCanisterSummary {
            fleet_registry: FleetRegistryVersion {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                                fleet_id: FleetId::from_generated_bytes([1; 32]),
                            },
                            app: AppId::from("toko"),
                        },
                        coordinator_subnet: SubnetId::from_principal(Principal::from_slice(
                            &[2; 29],
                        )),
                        coordinator: Principal::from_slice(&[3; 29]),
                    },
                    epoch: 1,
                },
                revision: 4,
                content_hash: [5; 32],
            },
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[6; 29])),
            fleet_subnet_root: Principal::from_slice(&[7; 29]),
            status: FleetSubnetRootStatus::Active,
            infrastructure_canisters: 2,
            component_canisters: 3,
            pooled_canisters: 4,
            total_canisters: 9,
        }
    }

    fn draining_response(
        summary: &FleetSubnetRootCanisterSummary,
    ) -> FleetSubnetRootDrainingResponse {
        FleetSubnetRootDrainingResponse {
            operation_id: [8; 32],
            fleet_subnet_root: summary.fleet_subnet_root,
            placement_subnet: summary.placement_subnet,
            active_registry: summary.fleet_registry.clone(),
            reservation_hash: [9; 32],
            component_topology_digest: ComponentTopologyDigest::from_bytes([10; 32]),
            active_release_set: FleetSubnetRootReleaseSet {
                release_build_id: crate::ids::ReleaseBuildId::from_nonce(
                    crate::ids::ReleaseBuildNonce::from_random_bytes([11; 32]),
                ),
                manifest_digest: crate::ids::ReleaseSetDigest::from_bytes([12; 32]),
            },
            next_allocation_sequence: 13,
            reserved_component_instances: 14,
            committed_component_instances: 15,
            managed_descendants: 16,
            known_created_component_canisters: 17,
            root_registry_encoded_bytes: 18_000,
            started_at_ns: 19,
        }
    }

    fn final_inventory_response(
        draining: &FleetSubnetRootDrainingResponse,
    ) -> FleetSubnetRootFinalInventoryResponse {
        FleetSubnetRootFinalInventoryResponse {
            operation_id: draining.operation_id,
            fleet_subnet_root: draining.fleet_subnet_root,
            placement_subnet: draining.placement_subnet,
            registry: draining.active_registry.clone(),
            component_topology_digest: draining.component_topology_digest,
            active_release_set: draining.active_release_set,
            next_allocation_sequence: draining.next_allocation_sequence,
            removed_component_instances: 12,
            terminal_component_history_hash: [19; 32],
            root_registry_encoded_bytes: 20_000,
            wasm_store: Principal::from_slice(&[21; 29]),
            wasm_store_catalog_hash: [22; 32],
            wasm_store_catalog_entries: 23,
            wasm_store_occupied_bytes: 24_000,
            wasm_store_template_count: 25,
            wasm_store_release_count: 26,
            wasm_store_gc_prepared_at_secs: 27,
            finalized_at_ns: 28,
            inventory_hash: [29; 32],
        }
    }
}
