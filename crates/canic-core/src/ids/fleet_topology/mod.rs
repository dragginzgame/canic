//! Module: ids::fleet_topology
//!
//! Responsibility: define protected Fleet topology, admission, limit, and binding facts.
//! Does not own: configuration compilation, placement decisions, Registry mutation, or storage.
//! Boundary: these passive cross-layer contracts are validated before authoritative use.

use crate::{
    cdk::types::Cycles,
    ids::{
        CanisterRole, ComponentInstanceId, ComponentSpecId, FleetBinding, ReleaseBuildId,
        ReleaseSetDigest, SubnetId,
    },
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::fmt;

///
/// ComponentTopologyDigest
///
/// SHA-256 identity of one canonical root-local Component Topology projection.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct ComponentTopologyDigest([u8; 32]);

impl ComponentTopologyDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ComponentTopologyDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

///
/// CyclesFundingBudget
///
/// Positive aggregate cycles-funding ceiling applied over one bounded window.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CyclesFundingBudget {
    pub window_secs: u64,
    pub maximum_cycles: Cycles,
}

/// Protected physical-topology class that selects the minimum Root funding baseline.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetFundingProfile {
    #[serde(rename = "single_subnet")]
    SingleSubnet,
    #[serde(rename = "multi_subnet")]
    MultiSubnet,
    #[serde(rename = "preview_multi_subnet")]
    PreviewMultiSubnet,
}

/// Minimum post-grant Coordinator execution reserve established by the 0.108 M0 proof.
pub const COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES: u128 = 100_000_000;

/// Conservative current-cost reservation for one bounded 16 KiB funding command.
pub const FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES: u128 = 42_118_809_000;

/// Minimum Root balance admitted for the Coordinator request and exact-retry path.
pub const FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES: u128 = 42_200_000_000;

/// Minimum Root balance admitted for automatic ICP-refill execution and recovery.
pub const FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES: u128 = 42_200_000_000;

/// Maximum registered roots represented by the bounded Coordinator funding ledger.
pub const MAX_FLEET_ROOT_FUNDING_SLOTS: usize = 4_096;

///
/// FleetCoordinatorRootFundingPolicy
///
/// Immutable Fleet-wide reserve and grant-budget authority installed into one Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCoordinatorRootFundingPolicy {
    pub funding_profile: FleetFundingProfile,
    pub minimum_reserve_cycles: Cycles,
    pub budget: CyclesFundingBudget,
    pub maximum_automatic_grants: u32,
    pub maximum_automatic_cycles: Cycles,
}

///
/// FleetSubnetRootFundingPolicy
///
/// Immutable Coordinator-grant thresholds and budget for one registered root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootFundingPolicy {
    pub funding_profile: FleetFundingProfile,
    pub request_threshold: Cycles,
    pub target_balance: Cycles,
    pub cooldown_secs: u64,
    pub budget: CyclesFundingBudget,
    pub maximum_automatic_grants: u32,
    pub maximum_automatic_cycles: Cycles,
}

///
/// FleetSubnetRootAutomaticIcpRefillPolicy
///
/// Optional emergency trigger and target subordinate to one root's ICP-refill policy.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootAutomaticIcpRefillPolicy {
    pub emergency_threshold: Cycles,
    pub target_balance: Cycles,
    pub maximum_automatic_refills: u32,
    pub maximum_automatic_refill_e8s: u64,
}

///
/// FleetSubnetRootIcpRefillPolicy
///
/// Immutable root-owned ICP conversion budget, balance floor, and system-Canister authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootIcpRefillPolicy {
    pub max_refill_e8s_per_call: u64,
    pub window_secs: u64,
    pub maximum_refill_e8s: u64,
    pub minimum_icp_balance_e8s: u64,
    pub min_xdr_permyriad_per_icp: Option<u64>,
    pub ledger_canister_id: Option<Principal>,
    pub cmc_canister_id: Option<Principal>,
    pub allow_ic_system_canister_overrides: bool,
    pub automatic: Option<FleetSubnetRootAutomaticIcpRefillPolicy>,
}

///
/// FleetSubnetRootFundingAuthority
///
/// Complete immutable Coordinator-grant and optional ICP-refill policy for one root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootFundingAuthority {
    pub root_funding: FleetSubnetRootFundingPolicy,
    pub icp_refill: Option<FleetSubnetRootIcpRefillPolicy>,
}

///
/// FleetSubnetCanisterPoolConfig
///
/// Immutable prepaid empty-Canister inventory policy for one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetCanisterPoolConfig {
    /// Ready empty Canisters automatically maintained for the root.
    pub minimum_size: u32,
    /// Ceiling for standby and operator-imported pool assets.
    ///
    /// Recycled assets remain tracked even when their return temporarily exceeds this target.
    pub maximum_size: u32,
    /// Minimum retained balance required before a pool asset becomes Ready.
    pub canister_cycles: Cycles,
    /// Native cycles retained above the Ready floor while a newly created asset is
    /// created, inspected, controller-checked, and admitted to the pool.
    pub creation_execution_margin: Cycles,
}

///
/// ComponentSpecAdmission
///
/// Immutable permission and concrete-instance ceiling for one Spec on one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSpecAdmission {
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub maximum_root_instances: u32,
}

///
/// FleetSubnetRootLimits
///
/// Immutable aggregate policy ceilings for one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootLimits {
    pub maximum_component_instances: u32,
    pub maximum_registry_bytes: u64,
    pub maximum_wasm_store_bytes: u64,
    pub canister_pool: FleetSubnetCanisterPoolConfig,
    pub cycles_funding: CyclesFundingBudget,
    /// Maximum accepted or committed Component Group placements on this root.
    pub maximum_group_placements: u32,
}

///
/// FleetCoordinatorBinding
///
/// Immutable Coordinator identity, placement and direct Fleet recovery authority.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCoordinatorBinding {
    pub fleet: FleetBinding,
    pub coordinator_subnet: SubnetId,
    pub coordinator: Principal,
    /// Canonical direct controllers that may recover any canister in this Fleet.
    pub recovery_controllers: Vec<Principal>,
}

impl FleetCoordinatorBinding {
    /// Exact controller set for a Root-owned canister, including recovery authority.
    #[must_use]
    pub fn root_controllers(&self, root: Principal) -> Vec<Principal> {
        let mut controllers = self.recovery_controllers.clone();
        controllers.push(root);
        controllers.sort_unstable();
        controllers.dedup();
        controllers
    }

    /// Compare IC controller sets without relying on management response order.
    #[must_use]
    pub fn has_exact_root_controllers(&self, root: Principal, observed: &[Principal]) -> bool {
        let mut observed = observed.to_vec();
        observed.sort_unstable();
        observed == self.root_controllers(root)
    }
}

///
/// FleetRegistryAuthority
///
/// Exact Coordinator binding and reinstall-local authority epoch for one Fleet Registry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetRegistryAuthority {
    pub binding: FleetCoordinatorBinding,
    pub epoch: u64,
}

///
/// FleetSubnetRootBinding
///
/// Complete immutable identity, placement, admissions, and limits of one Fleet Subnet Root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootBinding {
    pub authority: FleetRegistryAuthority,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
}

///
/// FleetSubnetWasmStoreAuthority
///
/// Exact reciprocal authority retained by one root and its host-installed sibling Store.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetWasmStoreAuthority {
    pub authority: FleetRegistryAuthority,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub installation_controller: Principal,
    pub release_build_id: ReleaseBuildId,
    pub wasm_module_hash: [u8; 32],
}

/// Exact child identity Root must use while activating its independently installed Store.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetWasmStoreActivationAuthority {
    pub fleet: FleetBinding,
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub release_build_id: ReleaseBuildId,
    pub component_topology_digest: ComponentTopologyDigest,
    pub controllers: Vec<Principal>,
    pub manifest_digest: ReleaseSetDigest,
}

///
/// ComponentBinding
///
/// Complete immutable identity and placement of one concrete Component.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentBinding {
    pub authority: FleetRegistryAuthority,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub canister_id: Principal,
}

///
/// ComponentChildBinding
///
/// Complete immutable identity of one child at any depth in one exact Component tree.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentChildBinding {
    pub component: ComponentBinding,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub canister_id: Principal,
}

///
/// ManagedCanisterBinding
///
/// Immutable Registry-issued identity retained by one managed application Canister.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ManagedCanisterBinding {
    Component(ComponentBinding),
    ComponentChild(ComponentChildBinding),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_controller_set_includes_exact_recovery_principals() {
        use crate::ids::{AppId, CanonicalNetworkId, FleetId, FleetKey};

        let root = Principal::from_slice(&[21; 29]);
        let recovery = Principal::from_slice(&[22; 29]);
        let binding = FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([23; 32]),
                },
                app: AppId::from("recovery-test"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[24; 29])),
            coordinator: Principal::from_slice(&[25; 29]),
            recovery_controllers: vec![recovery],
        };
        let mut expected = vec![root, recovery];
        expected.sort_unstable();
        assert_eq!(binding.root_controllers(root), expected);
        assert!(binding.has_exact_root_controllers(root, &[recovery, root]));
        assert!(!binding.has_exact_root_controllers(root, &[root]));
    }

    #[test]
    fn funding_profile_candid_spelling_roundtrips() {
        for profile in [
            FleetFundingProfile::SingleSubnet,
            FleetFundingProfile::PreviewMultiSubnet,
            FleetFundingProfile::MultiSubnet,
        ] {
            let bytes = candid::encode_one(profile).expect("encode funding profile Candid");
            assert_eq!(
                candid::decode_one::<FleetFundingProfile>(&bytes)
                    .expect("decode funding profile Candid"),
                profile
            );
        }
    }
}
