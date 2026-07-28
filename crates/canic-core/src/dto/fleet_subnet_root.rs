//! Module: dto::fleet_subnet_root
//!
//! Responsibility: carry protected fresh-install Fleet Subnet Root authority.
//! Does not own: validation, persistence, topology compilation, or lifecycle effects.
//! Boundary: the root lifecycle adapter passes init authority to workflow; controller queries
//! return the exact persisted authority without granting mutation rights.

use crate::{
    dto::fleet_registry::{FleetRegistryVersion, FleetSubnetRootStatus},
    ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet, SubnetId},
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
    fn canister_summary_round_trips_through_candid() {
        let summary = FleetSubnetRootCanisterSummary {
            fleet_registry: FleetRegistryVersion {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::public_ic(),
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
        };
        let candid = candid::encode_one(&summary).expect("encode Canister summary");
        let decoded: FleetSubnetRootCanisterSummary =
            candid::decode_one(&candid).expect("decode Canister summary");

        assert_eq!(decoded, summary);
    }
}
