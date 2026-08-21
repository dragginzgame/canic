//! Module: ops::fleet_funding_policy
//!
//! Responsibility: derive canonical identities for immutable Fleet root-funding policy.
//! Does not own: policy validation, storage, treasury accounting, or external effects.
//! Boundary: host and canister owners hash the same bounded protected policy shapes.

use crate::dto::fleet_registry::FleetRegistryVersion;
use crate::ids::{
    FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingAuthority,
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
    encoder.u128(policy.minimum_reserve_cycles.to_u128());
    encoder.u64(policy.budget.window_secs);
    encoder.u128(policy.budget.maximum_cycles.to_u128());
    encoder.finish()
}

/// Return the canonical digest of one root's complete immutable funding authority.
#[must_use]
pub fn fleet_subnet_root_funding_policy_hash(
    authority: &FleetSubnetRootFundingAuthority,
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(ROOT_POLICY_DOMAIN);
    encoder.u128(authority.root_funding.request_threshold.to_u128());
    encoder.u128(authority.root_funding.target_balance.to_u128());
    encoder.u64(authority.root_funding.cooldown_secs);
    encoder.u64(authority.root_funding.budget.window_secs);
    encoder.u128(authority.root_funding.budget.maximum_cycles.to_u128());
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
    } else {
        encoder.u8(0);
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
            CyclesFundingBudget, FleetSubnetRootAutomaticIcpRefillPolicy,
            FleetSubnetRootFundingPolicy,
        },
    };

    #[test]
    fn coordinator_policy_hash_is_stable_and_binds_every_field() {
        let policy = FleetCoordinatorRootFundingPolicy {
            minimum_reserve_cycles: Cycles::new(100_000_000),
            budget: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(10_000_000_000_000),
            },
        };
        let baseline = coordinator_root_funding_policy_hash(&policy);
        assert_eq!(
            crate::cdk::utils::hash::hex_bytes(baseline),
            "26a8f270734f672e87b68c6e6ca8b98df1001d7dd7cea985c979a6bdf4963618"
        );

        let mut changed = policy.clone();
        changed.minimum_reserve_cycles = Cycles::new(100_000_001);
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = policy.clone();
        changed.budget.window_secs += 1;
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));

        let mut changed = policy;
        changed.budget.maximum_cycles = Cycles::new(10_000_000_000_001);
        assert_ne!(baseline, coordinator_root_funding_policy_hash(&changed));
    }

    #[test]
    fn root_policy_hash_binds_every_decision_field() {
        let authority = authority();
        let baseline = fleet_subnet_root_funding_policy_hash(&authority);
        assert_eq!(
            crate::cdk::utils::hash::hex_bytes(baseline),
            "83eabbea6076f289519e734faf021f134ce7e4e028b16ddcff5af64f1dc6f40c"
        );

        let mut changed = authority.clone();
        changed.root_funding.request_threshold = Cycles::new(43_000_000_000);
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed.root_funding.target_balance = Cycles::new(61_000_000_000);
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed.root_funding.cooldown_secs += 1;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed.root_funding.budget.window_secs += 1;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed.root_funding.budget.maximum_cycles = Cycles::new(100_000_000_001);
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .max_refill_e8s_per_call += 1;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .window_secs += 1;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .maximum_refill_e8s += 1;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .minimum_icp_balance_e8s += 1;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .min_xdr_permyriad_per_icp = None;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .ledger_canister_id = Some(Principal::from_slice(&[13; 29]));
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .cmc_canister_id = Some(Principal::from_slice(&[14; 29]));
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .allow_ic_system_canister_overrides = false;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .automatic
            .as_mut()
            .expect("fixture automatic policy")
            .emergency_threshold = Cycles::new(42_300_000_000);
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority.clone();
        changed
            .icp_refill
            .as_mut()
            .expect("fixture ICP policy")
            .automatic
            .as_mut()
            .expect("fixture automatic policy")
            .target_balance = Cycles::new(31_000_000_000);
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));

        let mut changed = authority;
        changed.icp_refill = None;
        assert_ne!(baseline, fleet_subnet_root_funding_policy_hash(&changed));
    }

    fn authority() -> FleetSubnetRootFundingAuthority {
        FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                request_threshold: Cycles::new(50_000_000_000),
                target_balance: Cycles::new(60_000_000_000),
                cooldown_secs: 300,
                budget: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(100_000_000_000),
                },
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
                }),
            }),
        }
    }
}
