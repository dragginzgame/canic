//! Module: dto::component_registry
//!
//! Responsibility: carry root-local Component Registry preparation and allocation evidence.
//! Does not own: admission policy, stable mutation, artifact resolution, or lifecycle effects.
//! Boundary: callers name intent and Spec while the root allocates identity under verified authority.

use crate::{
    cdk::types::Cycles,
    dto::{fleet_registry::FleetRegistryVersion, root_store::RootStoreBootstrapRequest},
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        ComponentTopologyDigest, FleetSubnetRootReleaseSet,
    },
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// RootComponentRegistryPreparationRequest
///
/// Exact authority required before an empty root-local Component Registry may be prepared.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryPreparationRequest {
    pub store_bootstrap: RootStoreBootstrapRequest,
    pub expected_fleet_registry: FleetRegistryVersion,
}

///
/// RootComponentRegistryStatusResponse
///
/// Compact durable Component Registry authority and current allocation counters.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryStatusResponse {
    pub fleet_subnet_root: Principal,
    pub prepared_against_registry: FleetRegistryVersion,
    pub release_set: FleetSubnetRootReleaseSet,
    pub component_topology_digest: ComponentTopologyDigest,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub encoded_bytes: u64,
}

///
/// RootComponentAllocationRequest
///
/// Controller command naming one idempotent top-level Component reservation intent.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationRequest {
    pub operation_id: [u8; 32],
    pub component_spec: ComponentSpecId,
}

///
/// RootComponentAllocationStatusRequest
///
/// Read-only lookup key for one durable top-level Component allocation operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationStatusRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentCreationRequest
///
/// Controller command continuing one already reserved top-level Component operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCreationRequest {
    pub operation_id: [u8; 32],
}

///
/// RootComponentInstallRequest
///
/// Controller command continuing one already created top-level Component operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInstallRequest {
    pub operation_id: [u8; 32],
}

///
/// ComponentProvisioningOrigin
///
/// Authenticated causal authority retained with one top-level Component allocation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentProvisioningOrigin {
    FleetAdministrator { caller: Principal },
}

///
/// RootComponentAllocationPhase
///
/// Durable root-local progress of one top-level Component allocation operation.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentAllocationPhase {
    Reserved,
    CreationIntent,
    Created,
    InstallIntent,
    Installed,
    Verified,
}

///
/// RootComponentCreationEvidence
///
/// Exact Store artifact and root-owned creation settings frozen before the paid effect.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentCreationEvidence {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
    pub canister: Option<Principal>,
}

///
/// RootComponentInstallEvidence
///
/// Exact raw artifact, chunk source and immutable target binding frozen before installation.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentInstallEvidence {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
}

///
/// RootComponentAllocationResponse
///
/// Durable identity reservation returned identically for exact operation retry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentAllocationResponse {
    pub operation_id: [u8; 32],
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub phase: RootComponentAllocationPhase,
    pub creation: Option<RootComponentCreationEvidence>,
    pub installation: Option<RootComponentInstallEvidence>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::root_store::RootStoreBootstrapRequest,
        ids::{
            AppId, CanonicalNetworkId, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn component_registry_contracts_round_trip_through_candid() {
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: crate::ids::FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::public_ic(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("toko"),
                },
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2; 29])),
                coordinator: Principal::from_slice(&[3; 29]),
            },
            epoch: 1,
        };
        let request = RootComponentRegistryPreparationRequest {
            store_bootstrap: RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
            expected_fleet_registry: FleetRegistryVersion {
                authority,
                revision: 4,
                content_hash: [5; 32],
            },
        };
        let response = RootComponentRegistryStatusResponse {
            fleet_subnet_root: Principal::from_slice(&[6; 29]),
            prepared_against_registry: request.expected_fleet_registry.clone(),
            release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [7; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([8; 32]),
            },
            component_topology_digest: ComponentTopologyDigest::from_bytes([9; 32]),
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            encoded_bytes: 0,
        };
        let allocation = RootComponentAllocationResponse {
            operation_id: [10; 32],
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([11; 32]),
            component_spec: "projects".parse().expect("Component Spec ID"),
            spec_hash: [12; 32],
            role: CanisterRole::new("project_hub"),
            provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
                caller: Principal::from_slice(&[13; 29]),
            },
            release_set: response.release_set,
            phase: RootComponentAllocationPhase::Reserved,
            creation: None,
            installation: None,
        };
        let created = RootComponentAllocationResponse {
            phase: RootComponentAllocationPhase::Created,
            creation: Some(RootComponentCreationEvidence {
                wasm_store: Principal::from_slice(&[14; 29]),
                payload_hash: [15; 32],
                payload_size_bytes: 4_096,
                initial_cycles: Cycles::new(5_000_000_000_000),
                controller: Principal::from_slice(&[6; 29]),
                canister: Some(Principal::from_slice(&[16; 29])),
            }),
            installation: None,
            ..allocation.clone()
        };
        let request_bytes = candid::encode_one(&request).expect("encode request");
        let response_bytes = candid::encode_one(&response).expect("encode response");
        let allocation_bytes = candid::encode_one(&allocation).expect("encode allocation");
        let created_bytes = candid::encode_one(&created).expect("encode created allocation");

        assert_eq!(
            candid::decode_one::<RootComponentRegistryPreparationRequest>(&request_bytes)
                .expect("decode request"),
            request
        );
        assert_eq!(
            candid::decode_one::<RootComponentRegistryStatusResponse>(&response_bytes)
                .expect("decode response"),
            response
        );
        assert_eq!(
            candid::decode_one::<RootComponentAllocationResponse>(&allocation_bytes)
                .expect("decode allocation"),
            allocation
        );
        assert_eq!(
            candid::decode_one::<RootComponentAllocationResponse>(&created_bytes)
                .expect("decode created allocation"),
            created
        );
    }

    #[test]
    fn component_creation_request_round_trips_through_candid() {
        let request = RootComponentCreationRequest {
            operation_id: [10; 32],
        };
        let bytes = candid::encode_one(request).expect("encode creation request");

        assert_eq!(
            candid::decode_one::<RootComponentCreationRequest>(&bytes)
                .expect("decode creation request"),
            request
        );
    }

    #[test]
    fn component_install_request_round_trips_through_candid() {
        let request = RootComponentInstallRequest {
            operation_id: [10; 32],
        };
        let bytes = candid::encode_one(request).expect("encode install request");

        assert_eq!(
            candid::decode_one::<RootComponentInstallRequest>(&bytes)
                .expect("decode install request"),
            request
        );
    }
}
