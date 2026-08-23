//! Module: dto::fleet_funding
//!
//! Responsibility: carry bounded Root/Coordinator funding requests and exact results.
//! Does not own: caller identity, policy decisions, persistence, accounting, or effects.
//! Boundary: transport callers supply no recipient; the Coordinator derives it from the caller.

use crate::{
    cdk::types::Cycles,
    dto::fleet_registry::FleetRegistryVersion,
    ids::{FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingPolicy, SubnetId},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Frozen ingress envelope used by both sides of the funding command exchange.
pub const MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES: usize =
    crate::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES;

/// Maximum exact Root entries staged beneath one policy-rotation operation.
pub const MAX_FLEET_FUNDING_POLICY_ROTATION_ROOTS: usize = 4_096;

/// Maximum total Root-policy checkpoints retained across completed rotations.
pub const MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS: usize = 4_096;

/// Current-generation and retained predecessor automatic-funding usage.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyUsage {
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub generation_automatic_grants: u32,
    pub generation_automatic_cycles: Cycles,
}

/// Exact protected physical-placement evidence reviewed for one rotation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationPlacementEvidence {
    pub subnet: SubnetId,
    pub node_count: u64,
    pub cost_multiplier_numerator: u64,
    pub cost_multiplier_denominator: u64,
    pub fiduciary: bool,
    pub acknowledge_fiduciary_cost: bool,
}

/// Sole future value source authorized by a funding-policy rotation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetFundingPolicyRotationFundingSource {
    CoordinatorTreasury,
}

/// Immutable header of one no-effect operator-reviewed rotation plan.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationPlanHeader {
    pub predecessor_registry: FleetRegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub predecessor_coordinator_policy_hash: [u8; 32],
    pub predecessor_usage: FleetFundingPolicyUsage,
    pub proposed_coordinator_policy: FleetCoordinatorRootFundingPolicy,
    pub topology_catalog_digest: [u8; 32],
    pub coordinator_placement: FleetFundingPolicyRotationPlacementEvidence,
    pub affected_root_count: u32,
    pub roots_digest: [u8; 32],
    pub maximum_new_automatic_cycles: Cycles,
    pub apply_operator_debit: Cycles,
    pub funding_source: FleetFundingPolicyRotationFundingSource,
}

/// Exact current and proposed authority for one affected Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationRootPlan {
    pub fleet_subnet_root: Principal,
    pub predecessor_policy_hash: [u8; 32],
    pub predecessor_usage: FleetFundingPolicyUsage,
    pub proposed_policy: FleetSubnetRootFundingPolicy,
    pub placement: FleetFundingPolicyRotationPlacementEvidence,
}

/// Complete read-only rotation plan whose digest is accepted by apply.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationPlan {
    pub header: FleetFundingPolicyRotationPlanHeader,
    pub roots: Vec<FleetFundingPolicyRotationRootPlan>,
}

/// Begin staging one accepted no-effect rotation plan.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationBeginRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub header: FleetFundingPolicyRotationPlanHeader,
}

/// Stage one digest-bound Root policy beneath the current rotation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationStageRootRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub root: FleetFundingPolicyRotationRootPlan,
}

/// Start or exactly resume the fully staged rotation operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationApplyRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub expected_predecessor_generation: u64,
}

/// Coordinator-authored request that fences one Root beneath a staged plan.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationRootPrepareRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_registry: FleetRegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub root: FleetFundingPolicyRotationRootPlan,
}

/// Coordinator-authored request that converges one prepared Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationRootActivateRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_registry: FleetRegistryVersion,
    pub successor_registry: FleetRegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub fleet_subnet_root: Principal,
}

/// Idempotent Root receipt for one exact rotation phase.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationRootReceipt {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub prepared: bool,
    pub activated: bool,
    pub recorded_at_ns: u64,
}

/// Terminal Coordinator receipt for a converged policy generation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetFundingPolicyRotationReceipt {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_registry: FleetRegistryVersion,
    pub successor_registry: FleetRegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub affected_root_count: u32,
    pub retained_historical_automatic_grants: u64,
    pub retained_historical_automatic_cycles: Cycles,
    pub successor_policy_set_hash: [u8; 32],
    pub maximum_new_automatic_cycles: Cycles,
    pub apply_operator_debit: Cycles,
    pub completed_at_ns: u64,
}

/// Root-authored request for one exact Coordinator operating-cycle decision.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: FleetRegistryVersion,
    pub observed_balance: Cycles,
    pub requested_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// Coordinator-authored exact same-Root acceptance request carrying cycles.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingAcceptanceRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: FleetRegistryVersion,
    pub observed_balance: Cycles,
    pub granted_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// Root-authored exact receipt for one fresh or replayed grant acceptance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingAcceptanceReceipt {
    pub request: FleetRootFundingAcceptanceRequest,
    pub fleet_subnet_root: candid::Principal,
    pub coordinator: candid::Principal,
    pub accepted_at_ns: u64,
}

/// Terminal reason that one authenticated request transferred no cycles.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetRootFundingNoGrantReason {
    CooldownActive,
    CoordinatorReserveUnavailable,
    FleetAutomaticCapExhausted,
    FleetWindowExhausted,
    FundingDisabled,
    InvalidRequest,
    PolicyMismatch,
    RegistryStale,
    RootIneligible,
    RootRejected,
    RootAutomaticCapExhausted,
    RootWindowExhausted,
}

/// Durable exact zero-transfer result for one Root operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRootFundingNoGrantReceipt {
    pub request: FleetRootFundingRequest,
    pub reason: FleetRootFundingNoGrantReason,
    pub decided_at_ns: u64,
}

/// Exact terminal outcome returned to the authenticated requesting Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetRootFundingResponse {
    Granted(FleetRootFundingAcceptanceReceipt),
    NoGrant(FleetRootFundingNoGrantReceipt),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{
        AppId, CanonicalNetworkId, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetId, FleetKey,
        FleetRegistryAuthority, FleetSubnetRootFundingPolicy, SubnetId,
    };
    use candid::{Decode, Encode, Principal};

    #[test]
    fn exact_funding_commands_roundtrip_below_the_frozen_ingress_envelope() {
        let request = FleetRootFundingRequest {
            operation_id: [1; 32],
            operation_sequence: u64::MAX,
            expected_registry: registry(),
            observed_balance: Cycles::new(u128::MAX),
            requested_cycles: Cycles::new(u128::MAX),
            policy_hash: [2; 32],
        };
        let bytes = Encode!(&request).expect("encode Root funding request");
        assert!(bytes.len() <= MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES);
        assert_eq!(
            Decode!(&bytes, FleetRootFundingRequest).expect("decode Root funding request"),
            request
        );

        let acceptance = FleetRootFundingAcceptanceRequest {
            operation_id: request.operation_id,
            operation_sequence: request.operation_sequence,
            expected_registry: request.expected_registry,
            observed_balance: request.observed_balance,
            granted_cycles: request.requested_cycles,
            policy_hash: request.policy_hash,
        };
        let bytes = Encode!(&acceptance).expect("encode funding acceptance request");
        assert!(bytes.len() <= MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES);
        assert_eq!(
            Decode!(&bytes, FleetRootFundingAcceptanceRequest)
                .expect("decode funding acceptance request"),
            acceptance
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one complete maximum-size staged command envelope fixture"
    )]
    fn staged_rotation_commands_fit_the_frozen_ingress_envelope() {
        let registry = registry();
        let placement = FleetFundingPolicyRotationPlacementEvidence {
            subnet: SubnetId::from_principal(Principal::from_slice(&[31; 29])),
            node_count: u64::MAX,
            cost_multiplier_numerator: u64::MAX,
            cost_multiplier_denominator: u64::MAX,
            fiduciary: true,
            acknowledge_fiduciary_cost: true,
        };
        let usage = FleetFundingPolicyUsage {
            historical_automatic_grants: u64::MAX,
            historical_automatic_cycles: Cycles::new(u128::MAX),
            generation_automatic_grants: u32::MAX,
            generation_automatic_cycles: Cycles::new(u128::MAX),
        };
        let root = FleetFundingPolicyRotationRootPlan {
            fleet_subnet_root: Principal::from_slice(&[32; 29]),
            predecessor_policy_hash: [33; 32],
            predecessor_usage: usage.clone(),
            proposed_policy: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::PreviewMultiSubnet,
                request_threshold: Cycles::new(u128::MAX - 1),
                target_balance: Cycles::new(u128::MAX),
                cooldown_secs: u64::MAX,
                budget: CyclesFundingBudget {
                    window_secs: u64::MAX,
                    maximum_cycles: Cycles::new(u128::MAX),
                },
                maximum_automatic_grants: u32::MAX,
                maximum_automatic_cycles: Cycles::new(u128::MAX),
            },
            placement: placement.clone(),
        };
        let header = FleetFundingPolicyRotationPlanHeader {
            predecessor_registry: registry.clone(),
            predecessor_generation: u64::MAX - 1,
            successor_generation: u64::MAX,
            predecessor_coordinator_policy_hash: [34; 32],
            predecessor_usage: usage,
            proposed_coordinator_policy: FleetCoordinatorRootFundingPolicy {
                funding_profile: FleetFundingProfile::PreviewMultiSubnet,
                minimum_reserve_cycles: Cycles::new(u128::MAX),
                budget: CyclesFundingBudget {
                    window_secs: u64::MAX,
                    maximum_cycles: Cycles::new(u128::MAX),
                },
                maximum_automatic_grants: u32::MAX,
                maximum_automatic_cycles: Cycles::new(u128::MAX),
            },
            topology_catalog_digest: [35; 32],
            coordinator_placement: placement,
            affected_root_count: u32::try_from(MAX_FLEET_FUNDING_POLICY_ROTATION_ROOTS)
                .expect("rotation Root bound fits u32"),
            roots_digest: [36; 32],
            maximum_new_automatic_cycles: Cycles::new(u128::MAX),
            apply_operator_debit: Cycles::new(0),
            funding_source: FleetFundingPolicyRotationFundingSource::CoordinatorTreasury,
        };
        let commands = [
            Encode!(&FleetFundingPolicyRotationBeginRequest {
                operation_id: [37; 32],
                plan_digest: [38; 32],
                header,
            })
            .expect("encode begin"),
            Encode!(&FleetFundingPolicyRotationStageRootRequest {
                operation_id: [37; 32],
                plan_digest: [38; 32],
                root: root.clone(),
            })
            .expect("encode staged Root"),
            Encode!(&FleetFundingPolicyRotationApplyRequest {
                operation_id: [37; 32],
                plan_digest: [38; 32],
                expected_predecessor_generation: u64::MAX - 1,
            })
            .expect("encode apply"),
            Encode!(&FleetFundingPolicyRotationRootPrepareRequest {
                operation_id: [37; 32],
                plan_digest: [38; 32],
                predecessor_registry: registry.clone(),
                predecessor_generation: u64::MAX - 1,
                successor_generation: u64::MAX,
                root,
            })
            .expect("encode Root prepare"),
            Encode!(&FleetFundingPolicyRotationRootActivateRequest {
                operation_id: [37; 32],
                plan_digest: [38; 32],
                predecessor_registry: registry.clone(),
                successor_registry: registry,
                predecessor_generation: u64::MAX - 1,
                successor_generation: u64::MAX,
                fleet_subnet_root: Principal::from_slice(&[32; 29]),
            })
            .expect("encode Root activate"),
        ];
        assert!(
            commands
                .iter()
                .all(|bytes| { bytes.len() <= MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES })
        );
    }

    fn registry() -> FleetRegistryVersion {
        let coordinator = Principal::from_slice(&[3; 29]);
        FleetRegistryVersion {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([4; 32]),
                        },
                        app: AppId::from("funding-payload-proof"),
                    },
                    coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[5; 29])),
                    coordinator,
                },
                epoch: u64::MAX,
            },
            revision: u64::MAX,
            content_hash: [6; 32],
        }
    }
}
