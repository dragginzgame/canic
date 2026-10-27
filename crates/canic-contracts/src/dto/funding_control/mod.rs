//! Native funding commands and receipts; persisted policy and accounting retain their owners.
use crate::{
    cycles::Cycles,
    dto::{
        fleet_funding::{
            FleetFundingPolicyRotationFundingSource, FleetFundingPolicyUsage,
            FleetRootFundingNoGrantReason,
        },
        registry_discovery::RegistryVersion,
    },
    ids::{FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingPolicy},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// FleetFundingPolicyRotationPlacementEvidence transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PlacementEvidence {
    pub subnet: Principal,
    pub node_count: u64,
    pub cost_multiplier_numerator: u64,
    pub cost_multiplier_denominator: u64,
    pub fiduciary: bool,
    pub acknowledge_fiduciary_cost: bool,
}

/// FleetFundingPolicyRotationPlanHeader transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PlanHeader {
    pub predecessor_registry: RegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub predecessor_coordinator_policy_hash: [u8; 32],
    pub predecessor_usage: FleetFundingPolicyUsage,
    pub proposed_coordinator_policy: FleetCoordinatorRootFundingPolicy,
    pub topology_catalog_digest: [u8; 32],
    pub coordinator_placement: PlacementEvidence,
    pub affected_root_count: u32,
    pub roots_digest: [u8; 32],
    pub maximum_new_automatic_cycles: Cycles,
    pub apply_operator_debit: Cycles,
    pub funding_source: FleetFundingPolicyRotationFundingSource,
}

/// FleetFundingPolicyRotationRootPlan transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootPlan {
    pub fleet_subnet_root: Principal,
    pub predecessor_policy_hash: [u8; 32],
    pub predecessor_usage: FleetFundingPolicyUsage,
    pub proposed_policy: FleetSubnetRootFundingPolicy,
    pub placement: PlacementEvidence,
}

/// FleetFundingPolicyRotationBeginRequest transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct BeginRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub header: PlanHeader,
}

/// FleetFundingPolicyRotationStageRootRequest transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StageRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub root: RootPlan,
}

/// FleetFundingPolicyRotationRootPrepareRequest transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PrepareRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_registry: RegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub root: RootPlan,
}

/// FleetFundingPolicyRotationRootActivateRequest transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActivateRequest {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_registry: RegistryVersion,
    pub successor_registry: RegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub fleet_subnet_root: Principal,
}

/// FleetFundingPolicyRotationRootReceipt transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RotationReceipt {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub prepared: bool,
    pub activated: bool,
    pub recorded_at_ns: u64,
}

/// FleetRootFundingRequest transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Request {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: RegistryVersion,
    pub observed_balance: Cycles,
    pub requested_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// FleetRootFundingAcceptanceRequest transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct AcceptanceRequest {
    pub operation_id: [u8; 32],
    pub operation_sequence: u64,
    pub expected_registry: RegistryVersion,
    pub observed_balance: Cycles,
    pub granted_cycles: Cycles,
    pub policy_hash: [u8; 32],
}

/// FleetRootFundingAcceptanceReceipt transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct AcceptanceReceipt {
    pub request: AcceptanceRequest,
    pub fleet_subnet_root: Principal,
    pub coordinator: Principal,
    pub accepted_at_ns: u64,
}

/// FleetRootFundingNoGrantReceipt transported with native identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct NoGrantReceipt {
    pub request: Request,
    pub reason: FleetRootFundingNoGrantReason,
    pub decided_at_ns: u64,
}

/// Exact outcome of one authenticated Root funding request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Response {
    Granted(AcceptanceReceipt),
    NoGrant(NoGrantReceipt),
}
