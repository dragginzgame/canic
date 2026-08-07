//! Module: dto::component_provisioning
//!
//! Responsibility: carry one canonical Fleet Component provisioning plan across boundaries.
//! Does not own: plan derivation, validation, persistence, root effects, or receipts.
//! Boundary: the Coordinator retains the complete plan and sends each root only its exact batch.

use crate::{
    config::{ComponentDeploymentLabel, ComponentDeploymentLimits, ComponentDeploymentPurpose},
    dto::fleet_registry::FleetRegistryVersion,
    ids::{
        ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentGroupSpecId, ComponentSpecId,
        FleetBinding, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
    },
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Complete canonical provisioning authority retained before any root effect.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningPlan {
    pub fleet: FleetBinding,
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub operation: FleetComponentProvisioningOperation,
    pub directory_confirmation_roots: Vec<Principal>,
    pub batches: Vec<FleetSubnetRootProvisioningBatch>,
}

/// Controller-authenticated command that durably freezes one complete plan.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningPrepareRequest {
    pub operation_id: [u8; 32],
    pub plan: FleetComponentProvisioningPlan,
}

/// Controller-authenticated command advancing one exact Coordinator provisioning step.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningAdvanceRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub expected_accepted_root_count: u32,
    pub expected_provisioned_root_count: u32,
    pub expected_current_root: Option<FleetComponentProvisioningRootProgress>,
}

/// Exact root-local cursor copied from passive Coordinator status before one advance.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningRootProgress {
    pub fleet_subnet_root: Principal,
    pub component_count: u32,
    pub reserved_component_count: u32,
    pub claimed_component_count: u32,
    pub installed_component_count: u32,
    pub registry_committed_component_count: u32,
}

/// Exact passive lookup key for one Coordinator-owned provisioning operation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningStatusRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
}

/// Durable Coordinator progress exposed without returning the complete plan.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetComponentProvisioningPhase {
    Planned,
    AcceptingRoots,
    RootsAccepted,
    ProvisioningRoots,
    ComponentsProvisioned,
}

/// Compact exact status for one Coordinator-owned provisioning operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningStatusResponse {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub operation: FleetComponentProvisioningOperation,
    pub phase: FleetComponentProvisioningPhase,
    pub directory_confirmation_root_count: u32,
    pub root_batch_count: u32,
    pub accepted_root_count: u32,
    pub acceptance_in_flight_root: Option<Principal>,
    pub provisioned_root_count: u32,
    pub current_root: Option<FleetComponentProvisioningRootProgress>,
    pub provisioning_in_flight_root: Option<Principal>,
    pub group_placement_count: u32,
    pub component_count: u32,
    pub planned_at_ns: u64,
    pub roots_accepted_at_ns: Option<u64>,
    pub components_provisioned_at_ns: Option<u64>,
}

/// Fresh-install or monotonic scale-out scope covered by one plan.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum FleetComponentProvisioningOperation {
    FreshInstall,
    ScaleOut {
        deployment: ComponentGroupDeploymentId,
        previous_placements: u32,
        requested_placements: u32,
    },
}

/// One selected root's complete canonical provisioning batch.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootProvisioningBatch {
    pub root: FleetSubnetRootBinding,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub placements: Vec<ComponentGroupPlacementPlan>,
}

/// One materialized copy of a completely flattened Component Group.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupPlacementPlan {
    pub group_placement: ComponentGroupPlacementId,
    pub component_group: ComponentGroupSpecId,
    pub entries: Vec<ComponentGroupPlanEntry>,
}

/// One exact top-level Component occurrence within a group placement.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupPlanEntry {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub purpose: ComponentDeploymentPurpose,
    pub labels: Vec<ComponentDeploymentLabel>,
    pub limits: ComponentDeploymentLimits,
}

/// Coordinator-authenticated command asking one root to retain its exact plan batch.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentProvisioningAcceptanceRequest {
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub batch: FleetSubnetRootProvisioningBatch,
}

/// Read-only lookup key for one exact root provisioning operation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentProvisioningStatusRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
}

/// Coordinator-authenticated command advancing one exact bounded provisioning step.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentProvisioningAdvanceRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub expected_reserved_component_count: u32,
    pub expected_claimed_component_count: u32,
    pub expected_installed_component_count: u32,
    pub expected_registry_committed_component_count: u32,
}

/// One exact provisioned Component occurrence and its committed Registry identity.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootProvisionedGroupMember {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub purpose: ComponentDeploymentPurpose,
    pub limits: ComponentDeploymentLimits,
    pub binding: ComponentBinding,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
}

/// Complete provisioned result for one exact group placement on this root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootProvisionedGroupPlacement {
    pub group_placement: ComponentGroupPlacementId,
    pub component_group: ComponentGroupSpecId,
    pub members: Vec<RootProvisionedGroupMember>,
}

/// Complete group-partitioned result of one root provisioning operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentProvisioningResult {
    pub placements: Vec<RootProvisionedGroupPlacement>,
}

/// Durable aggregate progress of one root provisioning batch.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentProvisioningPhase {
    Accepted,
    Provisioned,
    Published,
    RuntimesActive,
}

/// Durable progress or complete terminal result for one exact root provisioning operation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentProvisioningStatusResponse {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub fleet_subnet_root: Principal,
    pub phase: RootComponentProvisioningPhase,
    pub placement_count: u32,
    pub component_count: u32,
    pub reserved_component_count: u32,
    pub claimed_component_count: u32,
    pub installed_component_count: u32,
    pub registry_committed_component_count: u32,
    pub result: Option<RootComponentProvisioningResult>,
    pub accepted_at_ns: u64,
    pub provisioned_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
}
