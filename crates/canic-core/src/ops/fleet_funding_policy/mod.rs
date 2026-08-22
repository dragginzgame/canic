//! Module: ops::fleet_funding_policy
//!
//! Responsibility: derive canonical identities for immutable Fleet root-funding policy.
//! Does not own: policy validation, storage, treasury accounting, or external effects.
//! Boundary: host and canister owners hash the same bounded protected policy shapes.

use crate::dto::fleet_registry::FleetRegistryVersion;
use crate::ids::{
    FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingAuthority,
    FleetSubnetRootIcpRefillPolicy,
};
use candid::Principal;
use sha2::{Digest, Sha256};

const COORDINATOR_POLICY_DOMAIN: &[u8] = b"canic/coordinator-root-funding-policy/v1";
const ROOT_POLICY_DOMAIN: &[u8] = b"canic/fleet-subnet-root-funding-policy/v1";
const ROOT_FUNDING_OPERATION_DOMAIN: &[u8] = b"canic/fleet-root-funding-operation/v1";

/// Return the canonical digest of one immutable Coordinator treasury policy.
#[must_use]
pub fn coordinator_root_funding_policy_hash(
    policy: &FleetCoordinatorRootFundingPolicy,
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(COORDINATOR_POLICY_DOMAIN);
    encoder.u8(funding_profile_tag(policy.funding_profile));
    encoder.u128(policy.minimum_reserve_cycles.to_u128());
    encoder.u64(policy.budget.window_secs);
    encoder.u128(policy.budget.maximum_cycles.to_u128());
    encoder.u32(policy.maximum_automatic_grants);
    encoder.u128(policy.maximum_automatic_cycles.to_u128());
    encoder.finish()
}

/// Return the canonical digest of one root's complete immutable funding authority.
#[must_use]
pub fn fleet_subnet_root_funding_policy_hash(
    authority: &FleetSubnetRootFundingAuthority,
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(ROOT_POLICY_DOMAIN);
    encoder.u8(funding_profile_tag(authority.root_funding.funding_profile));
    encoder.u128(authority.root_funding.request_threshold.to_u128());
    encoder.u128(authority.root_funding.target_balance.to_u128());
    encoder.u64(authority.root_funding.cooldown_secs);
    encoder.u64(authority.root_funding.budget.window_secs);
    encoder.u128(authority.root_funding.budget.maximum_cycles.to_u128());
    encoder.u32(authority.root_funding.maximum_automatic_grants);
    encoder.u128(authority.root_funding.maximum_automatic_cycles.to_u128());
    encode_icp_refill(&mut encoder, authority.icp_refill.as_ref());
    encoder.finish()
}

/// Derive one exact monotonic Root funding operation from caller-bound immutable facts.
#[must_use]
pub fn fleet_root_funding_operation_id(
    coordinator: Principal,
    fleet_subnet_root: Principal,
    operation_sequence: u64,
    registry: &FleetRegistryVersion,
    observed_balance: u128,
    requested_cycles: u128,
    policy_hash: [u8; 32],
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(ROOT_FUNDING_OPERATION_DOMAIN);
    encoder.bytes(coordinator.as_slice());
    encoder.bytes(fleet_subnet_root.as_slice());
    encoder.u64(operation_sequence);
    let authority = &registry.authority;
    encoder.bytes(
        authority
            .binding
            .fleet
            .fleet
            .canonical_network_id
            .as_bytes(),
    );
    encoder.bytes(authority.binding.fleet.fleet.fleet_id.as_bytes());
    encoder.bytes(authority.binding.fleet.app.as_str().as_bytes());
    encoder.bytes(
        authority
            .binding
            .coordinator_subnet
            .as_principal()
            .as_slice(),
    );
    encoder.bytes(authority.binding.coordinator.as_slice());
    encoder.u64(authority.epoch);
    encoder.u64(registry.revision);
    encoder.bytes(&registry.content_hash);
    encoder.u128(observed_balance);
    encoder.u128(requested_cycles);
    encoder.bytes(&policy_hash);
    encoder.finish()
}

fn encode_icp_refill(
    encoder: &mut CanonicalPolicyEncoder,
    policy: Option<&FleetSubnetRootIcpRefillPolicy>,
) {
    let Some(policy) = policy else {
        encoder.u8(0);
        return;
    };
    encoder.u8(1);
    encoder.u64(policy.max_refill_e8s_per_call);
    encoder.u64(policy.window_secs);
    encoder.u64(policy.maximum_refill_e8s);
    encoder.u64(policy.minimum_icp_balance_e8s);
    encoder.option_u64(policy.min_xdr_permyriad_per_icp);
    encoder.option_principal(policy.ledger_canister_id.as_ref());
    encoder.option_principal(policy.cmc_canister_id.as_ref());
    encoder.u8(u8::from(policy.allow_ic_system_canister_overrides));
    if let Some(automatic) = policy.automatic.as_ref() {
        encoder.u8(1);
        encoder.u128(automatic.emergency_threshold.to_u128());
        encoder.u128(automatic.target_balance.to_u128());
        encoder.u32(automatic.maximum_automatic_refills);
        encoder.u64(automatic.maximum_automatic_refill_e8s);
    } else {
        encoder.u8(0);
    }
}

const fn funding_profile_tag(profile: FleetFundingProfile) -> u8 {
    match profile {
        FleetFundingProfile::SingleSubnet => 1,
        FleetFundingProfile::MultiSubnet => 2,
        FleetFundingProfile::PreviewMultiSubnet => 3,
    }
}

struct CanonicalPolicyEncoder {
    bytes: Vec<u8>,
}

impl CanonicalPolicyEncoder {
    fn new(domain: &[u8]) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn option_principal(&mut self, value: Option<&Principal>) {
        if let Some(value) = value {
            self.u8(1);
            self.bytes(value.as_slice());
        } else {
            self.u8(0);
        }
    }

    fn option_u64(&mut self, value: Option<u64>) {
        if let Some(value) = value {
            self.u8(1);
            self.u64(value);
        } else {
            self.u8(0);
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self) -> [u8; 32] {
        Sha256::digest(self.bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        ids::{
            AppId, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetFundingProfile,
            FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootAutomaticIcpRefillPolicy,
            FleetSubnetRootFundingPolicy, SubnetId,
        },
    };

    #[test]
    fn coordinator_policy_hash_is_stable_and_binds_every_field() {
        let policy = FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            minimum_reserve_cycles: Cycles::new(100_000_000),
            budget: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(10_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(40_000_000_000_000),
        };
        let baseline = coordinator_root_funding_policy_hash(&policy);
        assert_eq!(
            crate::cdk::utils::hash::hex_bytes(baseline),
            "eec4a3e7156c9f43b9ef2d6ba120d6d43e09c3e411f22d49146d09cbbbfcf59e"
        );

        let mut changed = policy.clone();
        changed.funding_profile = FleetFundingProfile::MultiSubnet;
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = policy.clone();
        changed.minimum_reserve_cycles = Cycles::new(100_000_001);
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = policy.clone();
        changed.budget.window_secs += 1;
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = policy;
        changed.budget.maximum_cycles = Cycles::new(10_000_000_000_001);
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = coordinator_policy_fixture();
        changed.maximum_automatic_grants += 1;
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = coordinator_policy_fixture();
        changed.maximum_automatic_cycles = Cycles::new(40_000_000_000_001);
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));
    }

    #[test]
    fn root_policy_hash_is_stable() {
        let baseline = fleet_subnet_root_funding_policy_hash(&authority());
        assert_eq!(
            crate::cdk::utils::hash::hex_bytes(baseline),
            "60d20df682b4bc102675b57cde92c96fb1da083a80a9e2a2a457d2ac19016a56"
        );
    }

    #[test]
    fn root_policy_hash_binds_every_root_funding_field() {
        assert_authority_hash_changes(|value| {
            value.root_funding.funding_profile = FleetFundingProfile::MultiSubnet;
        });
        assert_authority_hash_changes(|value| {
            value.root_funding.request_threshold = Cycles::new(43_000_000_000);
        });
        assert_authority_hash_changes(|value| {
            value.root_funding.target_balance = Cycles::new(61_000_000_000);
        });
        assert_authority_hash_changes(|value| value.root_funding.cooldown_secs += 1);
        assert_authority_hash_changes(|value| value.root_funding.budget.window_secs += 1);
        assert_authority_hash_changes(|value| {
            value.root_funding.budget.maximum_cycles = Cycles::new(100_000_000_001);
        });
        assert_authority_hash_changes(|value| value.root_funding.maximum_automatic_grants += 1);
        assert_authority_hash_changes(|value| {
            value.root_funding.maximum_automatic_cycles = Cycles::new(240_000_000_001);
        });
    }

    #[test]
    fn root_policy_hash_binds_every_icp_refill_field() {
        assert_authority_hash_changes(|value| icp_policy(value).max_refill_e8s_per_call += 1);
        assert_authority_hash_changes(|value| icp_policy(value).window_secs += 1);
        assert_authority_hash_changes(|value| icp_policy(value).maximum_refill_e8s += 1);
        assert_authority_hash_changes(|value| icp_policy(value).minimum_icp_balance_e8s += 1);
        assert_authority_hash_changes(|value| {
            icp_policy(value).min_xdr_permyriad_per_icp = None;
        });
        assert_authority_hash_changes(|value| {
            icp_policy(value).ledger_canister_id = Some(Principal::from_slice(&[13; 29]));
        });
        assert_authority_hash_changes(|value| {
            icp_policy(value).cmc_canister_id = Some(Principal::from_slice(&[14; 29]));
        });
        assert_authority_hash_changes(|value| {
            icp_policy(value).allow_ic_system_canister_overrides = false;
        });
        assert_authority_hash_changes(|value| {
            automatic_policy(value).emergency_threshold = Cycles::new(42_300_000_000);
        });
        assert_authority_hash_changes(|value| {
            automatic_policy(value).target_balance = Cycles::new(31_000_000_000);
        });
        assert_authority_hash_changes(|value| {
            automatic_policy(value).maximum_automatic_refills += 1;
        });
        assert_authority_hash_changes(|value| {
            automatic_policy(value).maximum_automatic_refill_e8s += 1;
        });
        assert_authority_hash_changes(|value| value.icp_refill = None);
    }

    #[test]
    fn root_funding_operation_id_is_stable_and_caller_bound() {
        let coordinator = Principal::from_slice(&[1; 29]);
        let root = Principal::from_slice(&[2; 29]);
        let registry = FleetRegistryVersion {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: "00".repeat(32).parse().expect("network ID"),
                            fleet_id: FleetId::from_generated_bytes([6; 32]),
                        },
                        app: AppId::from("demo"),
                    },
                    coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[8; 29])),
                    coordinator,
                },
                epoch: 9,
            },
            revision: 7,
            content_hash: [3; 32],
        };
        let baseline = fleet_root_funding_operation_id(
            coordinator,
            root,
            7,
            &registry,
            42_000_000_000,
            18_000_000_000,
            [4; 32],
        );
        assert_eq!(
            crate::cdk::utils::hash::hex_bytes(baseline),
            "dbd4b14977e6ccc9a8e0e4c5241201ef185aa4330734416b1c26cad6858ecf34"
        );
        assert_ne!(
            baseline,
            fleet_root_funding_operation_id(
                coordinator,
                Principal::from_slice(&[5; 29]),
                7,
                &registry,
                42_000_000_000,
                18_000_000_000,
                [4; 32],
            )
        );
        assert_ne!(
            baseline,
            fleet_root_funding_operation_id(
                coordinator,
                root,
                8,
                &registry,
                42_000_000_000,
                18_000_000_000,
                [4; 32],
            )
        );
        let mut changed_registry = registry;
        changed_registry.content_hash[0] ^= 1;
        assert_ne!(
            baseline,
            fleet_root_funding_operation_id(
                coordinator,
                root,
                7,
                &changed_registry,
                42_000_000_000,
                18_000_000_000,
                [4; 32],
            )
        );
    }

    fn assert_authority_hash_changes(change: impl FnOnce(&mut FleetSubnetRootFundingAuthority)) {
        let mut changed = authority();
        let baseline = fleet_subnet_root_funding_policy_hash(&changed);
        change(&mut changed);
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));
    }

    fn icp_policy(
        authority: &mut FleetSubnetRootFundingAuthority,
    ) -> &mut FleetSubnetRootIcpRefillPolicy {
        authority.icp_refill.as_mut().expect("fixture ICP policy")
    }

    fn automatic_policy(
        authority: &mut FleetSubnetRootFundingAuthority,
    ) -> &mut FleetSubnetRootAutomaticIcpRefillPolicy {
        icp_policy(authority)
            .automatic
            .as_mut()
            .expect("fixture automatic policy")
    }

    fn authority() -> FleetSubnetRootFundingAuthority {
        FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::SingleSubnet,
                request_threshold: Cycles::new(50_000_000_000),
                target_balance: Cycles::new(60_000_000_000),
                cooldown_secs: 300,
                budget: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(100_000_000_000),
                },
                maximum_automatic_grants: 4,
                maximum_automatic_cycles: Cycles::new(240_000_000_000),
            },
            icp_refill: Some(FleetSubnetRootIcpRefillPolicy {
                max_refill_e8s_per_call: 100_000_000,
                window_secs: 86_400,
                maximum_refill_e8s: 200_000_000,
                minimum_icp_balance_e8s: 10_000_000,
                min_xdr_permyriad_per_icp: Some(40_000),
                ledger_canister_id: Some(Principal::from_slice(&[11; 29])),
                cmc_canister_id: Some(Principal::from_slice(&[12; 29])),
                allow_ic_system_canister_overrides: true,
                automatic: Some(FleetSubnetRootAutomaticIcpRefillPolicy {
                    emergency_threshold: Cycles::new(42_200_000_000),
                    target_balance: Cycles::new(55_000_000_000),
                    maximum_automatic_refills: 4,
                    maximum_automatic_refill_e8s: 400_000_000,
                }),
            }),
        }
    }

    fn coordinator_policy_fixture() -> FleetCoordinatorRootFundingPolicy {
        FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            minimum_reserve_cycles: Cycles::new(100_000_000),
            budget: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(10_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(40_000_000_000_000),
        }
    }
}
