//! Module: ops::fleet_funding_policy
//!
//! Responsibility: convert and validate immutable Fleet funding plans and derive their identities.
//! Does not own: storage, treasury accounting, orchestration, or external effects.
//! Boundary: boundary plans become DTO-free model input before one shared invariant decision.

use crate::dto::{
    fleet_funding::{
        FleetFundingPolicyRotationFundingSource, FleetFundingPolicyRotationPlacementEvidence,
        FleetFundingPolicyRotationPlan, FleetFundingPolicyRotationPlanHeader,
        FleetFundingPolicyRotationRootPlan, FleetFundingPolicyUsage,
    },
    fleet_registry::FleetRegistryVersion,
};
use crate::ids::{
    FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingAuthority,
    FleetSubnetRootFundingPolicy, FleetSubnetRootIcpRefillPolicy,
};
use crate::model::fleet_funding_policy::{
    FleetFundingPolicyRotationPlacementInput, FleetFundingPolicyRotationPlanInput,
    FleetFundingPolicyRotationRootInput, FleetFundingPolicyRotationUsageInput,
    FleetFundingPolicyRotationValidationError, validate_fleet_funding_policy_rotation,
};
use candid::Principal;
use sha2::{Digest, Sha256};

const COORDINATOR_POLICY_DOMAIN: &[u8] = b"canic/coordinator-root-funding-policy/v1";
const ROOT_POLICY_DOMAIN: &[u8] = b"canic/fleet-subnet-root-funding-policy/v1";
const ROOT_FUNDING_OPERATION_DOMAIN: &[u8] = b"canic/fleet-root-funding-operation/v1";
const FUNDING_ROTATION_ROOTS_DOMAIN: &[u8] = b"canic/funding-policy-rotation-roots/v1";
const FUNDING_ROTATION_PLAN_DOMAIN: &[u8] = b"canic/funding-policy-rotation-plan/v1";
const FUNDING_ROTATION_OPERATION_DOMAIN: &[u8] = b"canic/funding-policy-rotation-operation/v1";
const FUNDING_ROTATION_SUCCESSOR_POLICY_SET_DOMAIN: &[u8] =
    b"canic/funding-policy-rotation-successor-policy-set/v1";

/// Validate one boundary plan through the DTO-free model-owned invariants.
pub fn validate_fleet_funding_policy_rotation_plan(
    plan: &FleetFundingPolicyRotationPlan,
) -> Result<(), FleetFundingPolicyRotationValidationError> {
    let roots = plan
        .roots
        .iter()
        .map(|root| FleetFundingPolicyRotationRootInput {
            fleet_subnet_root: root.fleet_subnet_root,
            predecessor_usage: rotation_usage_input(&root.predecessor_usage),
            proposed_policy: &root.proposed_policy,
            placement: rotation_placement_input(&root.placement),
        })
        .collect();
    let header = &plan.header;
    let input = FleetFundingPolicyRotationPlanInput {
        predecessor_generation: header.predecessor_generation,
        successor_generation: header.successor_generation,
        predecessor_usage: rotation_usage_input(&header.predecessor_usage),
        proposed_coordinator_policy: &header.proposed_coordinator_policy,
        coordinator_placement: rotation_placement_input(&header.coordinator_placement),
        affected_root_count: header.affected_root_count,
        maximum_new_automatic_cycles: header.maximum_new_automatic_cycles.to_u128(),
        apply_operator_debit: header.apply_operator_debit.to_u128(),
        funding_source_is_coordinator_treasury: matches!(
            header.funding_source,
            FleetFundingPolicyRotationFundingSource::CoordinatorTreasury
        ),
        roots,
    };
    validate_fleet_funding_policy_rotation(&input)
}

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

/// Return the canonical digest of the exact sorted Root portion of one rotation plan.
#[must_use]
pub fn fleet_funding_policy_rotation_roots_digest(
    roots: &[FleetFundingPolicyRotationRootPlan],
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(FUNDING_ROTATION_ROOTS_DOMAIN);
    encoder.u64(roots.len() as u64);
    for root in roots {
        encode_rotation_root(&mut encoder, root);
    }
    encoder.finish()
}

/// Return the canonical digest accepted by policy-rotation apply.
#[must_use]
pub fn fleet_funding_policy_rotation_plan_digest(
    plan: &FleetFundingPolicyRotationPlan,
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(FUNDING_ROTATION_PLAN_DOMAIN);
    encode_rotation_header(&mut encoder, &plan.header);
    encoder.u64(plan.roots.len() as u64);
    for root in &plan.roots {
        encode_rotation_root(&mut encoder, root);
    }
    encoder.finish()
}

/// Derive the sole Coordinator rotation operation identity from its accepted plan.
#[must_use]
pub fn fleet_funding_policy_rotation_operation_id(
    coordinator: Principal,
    plan_digest: [u8; 32],
) -> [u8; 32] {
    let mut encoder = CanonicalPolicyEncoder::new(FUNDING_ROTATION_OPERATION_DOMAIN);
    encoder.bytes(coordinator.as_slice());
    encoder.bytes(&plan_digest);
    encoder.finish()
}

/// Commit the exact successor Coordinator and full per-Root funding authority set.
#[must_use]
pub fn fleet_funding_policy_rotation_successor_policy_set_hash<'a>(
    coordinator: &FleetCoordinatorRootFundingPolicy,
    roots: impl IntoIterator<Item = (Principal, &'a FleetSubnetRootFundingAuthority)>,
) -> [u8; 32] {
    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort_by_key(|(root, _)| *root);
    let mut encoder = CanonicalPolicyEncoder::new(FUNDING_ROTATION_SUCCESSOR_POLICY_SET_DOMAIN);
    encode_coordinator_policy(&mut encoder, coordinator);
    encoder.u64(roots.len() as u64);
    for (root, authority) in roots {
        encoder.bytes(root.as_slice());
        encode_root_policy(&mut encoder, &authority.root_funding);
        encode_icp_refill(&mut encoder, authority.icp_refill.as_ref());
    }
    encoder.finish()
}

const fn rotation_usage_input(
    usage: &FleetFundingPolicyUsage,
) -> FleetFundingPolicyRotationUsageInput {
    FleetFundingPolicyRotationUsageInput {
        historical_automatic_grants: usage.historical_automatic_grants,
        historical_automatic_cycles: usage.historical_automatic_cycles.to_u128(),
        generation_automatic_grants: usage.generation_automatic_grants,
        generation_automatic_cycles: usage.generation_automatic_cycles.to_u128(),
    }
}

const fn rotation_placement_input(
    placement: &FleetFundingPolicyRotationPlacementEvidence,
) -> FleetFundingPolicyRotationPlacementInput {
    FleetFundingPolicyRotationPlacementInput {
        subnet: placement.subnet,
        node_count: placement.node_count,
        cost_multiplier_numerator: placement.cost_multiplier_numerator,
        cost_multiplier_denominator: placement.cost_multiplier_denominator,
        fiduciary: placement.fiduciary,
        acknowledge_fiduciary_cost: placement.acknowledge_fiduciary_cost,
    }
}

fn encode_rotation_header(
    encoder: &mut CanonicalPolicyEncoder,
    header: &FleetFundingPolicyRotationPlanHeader,
) {
    encode_registry_version(encoder, &header.predecessor_registry);
    encoder.u64(header.predecessor_generation);
    encoder.u64(header.successor_generation);
    encoder.bytes(&header.predecessor_coordinator_policy_hash);
    encode_usage(encoder, &header.predecessor_usage);
    encode_coordinator_policy(encoder, &header.proposed_coordinator_policy);
    encoder.bytes(&header.topology_catalog_digest);
    encode_placement(encoder, &header.coordinator_placement);
    encoder.u32(header.affected_root_count);
    encoder.bytes(&header.roots_digest);
    encoder.u128(header.maximum_new_automatic_cycles.to_u128());
    encoder.u128(header.apply_operator_debit.to_u128());
    encoder.u8(match header.funding_source {
        FleetFundingPolicyRotationFundingSource::CoordinatorTreasury => 1,
    });
}

fn encode_rotation_root(
    encoder: &mut CanonicalPolicyEncoder,
    root: &FleetFundingPolicyRotationRootPlan,
) {
    encoder.bytes(root.fleet_subnet_root.as_slice());
    encoder.bytes(&root.predecessor_policy_hash);
    encode_usage(encoder, &root.predecessor_usage);
    encode_root_policy(encoder, &root.proposed_policy);
    encode_placement(encoder, &root.placement);
}

fn encode_registry_version(encoder: &mut CanonicalPolicyEncoder, registry: &FleetRegistryVersion) {
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
}

fn encode_usage(encoder: &mut CanonicalPolicyEncoder, usage: &FleetFundingPolicyUsage) {
    encoder.u64(usage.historical_automatic_grants);
    encoder.u128(usage.historical_automatic_cycles.to_u128());
    encoder.u32(usage.generation_automatic_grants);
    encoder.u128(usage.generation_automatic_cycles.to_u128());
}

fn encode_placement(
    encoder: &mut CanonicalPolicyEncoder,
    placement: &FleetFundingPolicyRotationPlacementEvidence,
) {
    encoder.bytes(placement.subnet.as_principal().as_slice());
    encoder.u64(placement.node_count);
    encoder.u64(placement.cost_multiplier_numerator);
    encoder.u64(placement.cost_multiplier_denominator);
    encoder.u8(u8::from(placement.fiduciary));
    encoder.u8(u8::from(placement.acknowledge_fiduciary_cost));
}

fn encode_coordinator_policy(
    encoder: &mut CanonicalPolicyEncoder,
    policy: &FleetCoordinatorRootFundingPolicy,
) {
    encoder.u8(funding_profile_tag(policy.funding_profile));
    encoder.u128(policy.minimum_reserve_cycles.to_u128());
    encoder.u64(policy.budget.window_secs);
    encoder.u128(policy.budget.maximum_cycles.to_u128());
    encoder.u32(policy.maximum_automatic_grants);
    encoder.u128(policy.maximum_automatic_cycles.to_u128());
}

fn encode_root_policy(encoder: &mut CanonicalPolicyEncoder, policy: &FleetSubnetRootFundingPolicy) {
    encoder.u8(funding_profile_tag(policy.funding_profile));
    encoder.u128(policy.request_threshold.to_u128());
    encoder.u128(policy.target_balance.to_u128());
    encoder.u64(policy.cooldown_secs);
    encoder.u64(policy.budget.window_secs);
    encoder.u128(policy.budget.maximum_cycles.to_u128());
    encoder.u32(policy.maximum_automatic_grants);
    encoder.u128(policy.maximum_automatic_cycles.to_u128());
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
            AppId, CanonicalNetworkId, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
            FleetFundingProfile, FleetId, FleetKey, FleetRegistryAuthority,
            FleetSubnetRootAutomaticIcpRefillPolicy, FleetSubnetRootFundingPolicy, SubnetId,
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
        let multi_subnet = coordinator_root_funding_policy_hash(&changed);
        assert_ne!(baseline, multi_subnet);

        let mut changed = policy.clone();
        changed.funding_profile = FleetFundingProfile::PreviewMultiSubnet;
        let preview_multi_subnet = coordinator_root_funding_policy_hash(&changed);
        assert_ne!(baseline, preview_multi_subnet);
        assert_ne!(multi_subnet, preview_multi_subnet);

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
            value.root_funding.funding_profile = FleetFundingProfile::PreviewMultiSubnet;
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

    #[test]
    fn rotation_plan_digest_binds_authority_usage_topology_and_successor_policy() {
        let plan = rotation_plan_fixture();
        let baseline = fleet_funding_policy_rotation_plan_digest(&plan);

        let mut changed = plan.clone();
        changed.header.predecessor_registry.content_hash[0] ^= 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan.clone();
        changed.header.successor_generation += 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan.clone();
        changed.header.predecessor_usage.historical_automatic_grants += 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan.clone();
        changed
            .header
            .proposed_coordinator_policy
            .maximum_automatic_grants += 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan.clone();
        changed.header.topology_catalog_digest[0] ^= 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan.clone();
        changed.header.coordinator_placement.node_count += 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan.clone();
        changed.roots[0]
            .predecessor_usage
            .generation_automatic_grants += 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let mut changed = plan;
        changed.roots[0].proposed_policy.maximum_automatic_grants += 1;
        assert_ne!(
            baseline,
            fleet_funding_policy_rotation_plan_digest(&changed)
        );

        let coordinator = Principal::from_slice(&[21; 29]);
        let operation = fleet_funding_policy_rotation_operation_id(coordinator, baseline);
        assert_ne!(
            operation,
            fleet_funding_policy_rotation_operation_id(Principal::from_slice(&[22; 29]), baseline)
        );
    }

    #[test]
    fn rotation_plan_is_bounded_monotonic_and_retains_exact_usage() {
        let plan = rotation_plan_fixture();
        validate_fleet_funding_policy_rotation_plan(&plan).expect("valid rotation plan");

        let mut changed = plan.clone();
        changed.header.successor_generation += 1;
        assert_eq!(
            validate_fleet_funding_policy_rotation_plan(&changed),
            Err(FleetFundingPolicyRotationValidationError::GenerationMismatch)
        );

        let mut changed = plan.clone();
        changed.header.predecessor_usage.generation_automatic_grants += 1;
        assert_eq!(
            validate_fleet_funding_policy_rotation_plan(&changed),
            Err(FleetFundingPolicyRotationValidationError::UsageMismatch)
        );

        let mut changed = plan.clone();
        changed.roots[1].fleet_subnet_root = changed.roots[0].fleet_subnet_root;
        assert_eq!(
            validate_fleet_funding_policy_rotation_plan(&changed),
            Err(FleetFundingPolicyRotationValidationError::RootOrderInvalid)
        );

        let mut changed = plan.clone();
        changed.header.apply_operator_debit = Cycles::new(1);
        assert_eq!(
            validate_fleet_funding_policy_rotation_plan(&changed),
            Err(FleetFundingPolicyRotationValidationError::OperatorDebitNonzero)
        );

        let mut changed = plan;
        changed.roots[0].placement.acknowledge_fiduciary_cost = true;
        assert_eq!(
            validate_fleet_funding_policy_rotation_plan(&changed),
            Err(FleetFundingPolicyRotationValidationError::PlacementEvidenceInvalid)
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

    #[expect(
        clippy::too_many_lines,
        reason = "one valid cross-Subnet boundary fixture makes every policy field explicit"
    )]
    fn rotation_plan_fixture() -> FleetFundingPolicyRotationPlan {
        const TC: u128 = 1_000_000_000_000;
        const THIRTY_DAYS: u64 = 30 * 24 * 60 * 60;
        const NINETY_DAYS: u64 = 90 * 24 * 60 * 60;
        let coordinator = Principal::from_slice(&[21; 29]);
        let subnet = SubnetId::from_principal(Principal::from_slice(&[23; 29]));
        let root_subnet = SubnetId::from_principal(Principal::from_slice(&[24; 29]));
        let registry = FleetRegistryVersion {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([25; 32]),
                        },
                        app: AppId::from("rotation-hash"),
                    },
                    coordinator_subnet: subnet,
                    coordinator,
                },
                epoch: 3,
            },
            revision: 4,
            content_hash: [26; 32],
        };
        let root_usage = FleetFundingPolicyUsage {
            historical_automatic_grants: 5,
            historical_automatic_cycles: Cycles::new(6),
            generation_automatic_grants: 2,
            generation_automatic_cycles: Cycles::new(7),
        };
        let placement = FleetFundingPolicyRotationPlacementEvidence {
            subnet,
            node_count: 13,
            cost_multiplier_numerator: 1,
            cost_multiplier_denominator: 1,
            fiduciary: false,
            acknowledge_fiduciary_cost: false,
        };
        let root_policy = FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::PreviewMultiSubnet,
            request_threshold: Cycles::new(10 * TC),
            target_balance: Cycles::new(30 * TC),
            cooldown_secs: THIRTY_DAYS,
            budget: CyclesFundingBudget {
                window_secs: NINETY_DAYS,
                maximum_cycles: Cycles::new(30 * TC),
            },
            maximum_automatic_grants: 2,
            maximum_automatic_cycles: Cycles::new(60 * TC),
        };
        let roots = vec![
            FleetFundingPolicyRotationRootPlan {
                fleet_subnet_root: Principal::from_slice(&[27; 29]),
                predecessor_policy_hash: [28; 32],
                predecessor_usage: root_usage.clone(),
                proposed_policy: root_policy.clone(),
                placement: placement.clone(),
            },
            FleetFundingPolicyRotationRootPlan {
                fleet_subnet_root: Principal::from_slice(&[29; 29]),
                predecessor_policy_hash: [30; 32],
                predecessor_usage: root_usage,
                proposed_policy: root_policy,
                placement: FleetFundingPolicyRotationPlacementEvidence {
                    subnet: root_subnet,
                    node_count: 13,
                    cost_multiplier_numerator: 1,
                    cost_multiplier_denominator: 1,
                    fiduciary: false,
                    acknowledge_fiduciary_cost: false,
                },
            },
        ];
        let mut plan = FleetFundingPolicyRotationPlan {
            header: FleetFundingPolicyRotationPlanHeader {
                predecessor_registry: registry,
                predecessor_generation: 8,
                successor_generation: 9,
                predecessor_coordinator_policy_hash: [31; 32],
                predecessor_usage: FleetFundingPolicyUsage {
                    historical_automatic_grants: 10,
                    historical_automatic_cycles: Cycles::new(12),
                    generation_automatic_grants: 4,
                    generation_automatic_cycles: Cycles::new(14),
                },
                proposed_coordinator_policy: FleetCoordinatorRootFundingPolicy {
                    funding_profile: FleetFundingProfile::PreviewMultiSubnet,
                    minimum_reserve_cycles: Cycles::new(80 * TC),
                    budget: CyclesFundingBudget {
                        window_secs: NINETY_DAYS,
                        maximum_cycles: Cycles::new(60 * TC),
                    },
                    maximum_automatic_grants: 4,
                    maximum_automatic_cycles: Cycles::new(120 * TC),
                },
                topology_catalog_digest: [32; 32],
                coordinator_placement: placement,
                affected_root_count: 2,
                roots_digest: [0; 32],
                maximum_new_automatic_cycles: Cycles::new(120 * TC),
                apply_operator_debit: Cycles::new(0),
                funding_source: FleetFundingPolicyRotationFundingSource::CoordinatorTreasury,
            },
            roots,
        };
        plan.header.roots_digest = fleet_funding_policy_rotation_roots_digest(&plan.roots);
        plan
    }
}
