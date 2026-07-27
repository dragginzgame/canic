//! Module: dto::component_registry
//!
//! Responsibility: carry root-local Component Registry preparation evidence.
//! Does not own: admission validation, stable mutation, identity allocation, or lifecycle effects.
//! Boundary: the host binds preparation to the already verified Store and active Fleet Registry.

use crate::{
    dto::{fleet_registry::FleetRegistryVersion, root_store::RootStoreBootstrapRequest},
    ids::{ComponentTopologyDigest, FleetSubnetRootReleaseSet},
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
    fn component_registry_preparation_contracts_round_trip_through_candid() {
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

        let request_bytes = candid::encode_one(&request).expect("encode request");
        let response_bytes = candid::encode_one(&response).expect("encode response");

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
    }
}
