//! Module: dto::fleet_subnet_root
//!
//! Responsibility: carry protected Fleet Subnet Root authority and controller lifecycle DTOs.
//! Does not own: validation, persistence, topology compilation, or lifecycle effects.
//! Boundary: lifecycle adapters pass init/command authority to workflow and return passive data.

use crate::{
    dto::fleet_registry::{FleetRegistryVersion, FleetSubnetRootStatus},
    ids::{ComponentTopologyDigest, FleetSubnetRootBinding, FleetSubnetRootReleaseSet, SubnetId},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

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
            total_canisters: 5,
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
            component_topology_digest: ComponentTopologyDigest::from_bytes([9; 32]),
            active_release_set: FleetSubnetRootReleaseSet {
                release_build_id: crate::ids::ReleaseBuildId::from_nonce(
                    crate::ids::ReleaseBuildNonce::from_random_bytes([10; 32]),
                ),
                manifest_digest: crate::ids::ReleaseSetDigest::from_bytes([11; 32]),
            },
            next_allocation_sequence: 12,
            reserved_component_instances: 13,
            committed_component_instances: 14,
            managed_descendants: 15,
            known_created_component_canisters: 16,
            root_registry_encoded_bytes: 17_000,
            started_at_ns: 18,
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
