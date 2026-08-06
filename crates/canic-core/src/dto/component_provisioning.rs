//! Module: dto::component_provisioning
//!
//! Responsibility: carry one canonical Fleet Component provisioning plan across boundaries.
//! Does not own: plan derivation, validation, persistence, root effects, or receipts.
//! Boundary: the Coordinator retains the complete plan and sends each root only its exact batch.

use crate::{
    config::{ComponentDeploymentLabel, ComponentDeploymentLimits, ComponentDeploymentPurpose},
    dto::fleet_registry::FleetRegistryVersion,
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
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

/// Coordinator-authenticated command advancing one exact bounded reservation step.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentProvisioningAdvanceRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub expected_reserved_component_count: u32,
}

/// Durable aggregate progress of one root provisioning batch.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentProvisioningPhase {
    Accepted,
    Provisioned,
    Published,
    RuntimesActive,
}

/// Compact receipt for one exact durable root provisioning operation.
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
    pub accepted_at_ns: u64,
    pub receipt_content_hash: [u8; 32],
}
