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
    pub expected_phase: FleetComponentProvisioningPhase,
    pub expected_accepted_root_count: u32,
    pub expected_provisioned_root_count: u32,
    pub expected_current_root: Option<FleetComponentProvisioningRootProgress>,
    pub expected_directory_confirmed_root_count: u32,
    pub expected_current_synchronization: Option<FleetComponentSynchronizationRootProgress>,
    pub expected_current_publication: Option<FleetComponentPublicationRootProgress>,
    pub expected_runtime_activated_root_count: u32,
    pub expected_current_activation: Option<FleetComponentActivationRootProgress>,
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

/// Exact root-local Directory cursor copied from passive Coordinator status.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentPublicationRootProgress {
    pub fleet_subnet_root: Principal,
    pub component_count: u32,
    pub published_component_count: u32,
}

/// Exact affected-service synchronization cursor copied from passive Coordinator status.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentSynchronizationRootProgress {
    pub fleet_subnet_root: Principal,
    pub affected_component_count: u32,
    pub synchronized_component_count: u32,
    pub complete: bool,
}

/// Exact root-local runtime-activation cursor copied from passive Coordinator status.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentActivationRootProgress {
    pub fleet_subnet_root: Principal,
    pub component_count: u32,
    pub activated_component_count: u32,
    pub root_runtime_active: bool,
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
    ServiceTopologyPublished,
    ConfirmingDirectories,
    DirectoriesConfirmed,
    ActivatingRuntimes,
    RuntimesActivated,
}

/// Exact Coordinator step whose current Root call most recently failed.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetComponentProvisioningRetryStage {
    RootAcceptance,
    RootProvisioning,
    DirectoryConfirmation,
    RuntimeActivation,
}

/// Bounded typed diagnostic retained for the Root call that remains retryable.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetComponentProvisioningRootFailure {
    pub fleet_subnet_root: Principal,
    pub stage: FleetComponentProvisioningRetryStage,
    pub diagnostic_code: u16,
    pub failed_at_ns: u64,
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
    pub directory_confirmed_root_count: u32,
    pub current_synchronization: Option<FleetComponentSynchronizationRootProgress>,
    pub current_publication: Option<FleetComponentPublicationRootProgress>,
    pub publication_in_flight_root: Option<Principal>,
    pub runtime_activated_root_count: u32,
    pub current_activation: Option<FleetComponentActivationRootProgress>,
    pub activation_in_flight_root: Option<Principal>,
    pub pending_root_failure: Option<FleetComponentProvisioningRootFailure>,
    pub group_placement_count: u32,
    pub component_count: u32,
    pub planned_at_ns: u64,
    pub roots_accepted_at_ns: Option<u64>,
    pub components_provisioned_at_ns: Option<u64>,
    pub published_fleet_registry: Option<FleetRegistryVersion>,
    pub service_topology_published_at_ns: Option<u64>,
    pub directories_confirmed_at_ns: Option<u64>,
    pub runtimes_activated_at_ns: Option<u64>,
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

/// One exact member of a root-derived Component Group Directory.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDirectoryMember {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub purpose: ComponentDeploymentPurpose,
    pub labels: Vec<ComponentDeploymentLabel>,
    pub binding: ComponentBinding,
}

/// Protected origin of one root-derived Component Group Directory.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDirectoryProvenance {
    pub authority: crate::ids::FleetRegistryAuthority,
    pub fleet_subnet_root: Principal,
    pub group_placement: ComponentGroupPlacementId,
    pub component_group: ComponentGroupSpecId,
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub placement_receipt_content_hash: [u8; 32],
}

/// Complete bounded sibling projection for one materialized Component Group placement.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDirectory {
    pub provenance: ComponentGroupDirectoryProvenance,
    pub members: Vec<ComponentGroupDirectoryMember>,
}

/// Compact proof of one exact Component Directory delivered during publication.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDirectoryPublicationEvidence {
    pub component: crate::ids::ComponentInstanceId,
    pub content_hash: [u8; 32],
}

/// Compact proof of one exact Component Group Directory delivered during publication.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupDirectoryPublicationEvidence {
    pub group_placement: ComponentGroupPlacementId,
    pub content_hash: [u8; 32],
}

/// Complete root-local evidence for one published provisioning batch.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentPublicationEvidence {
    pub fleet_registry: FleetRegistryVersion,
    pub fleet_directory_content_hash: [u8; 32],
    pub component_directories: Vec<ComponentDirectoryPublicationEvidence>,
    pub component_group_directories: Vec<ComponentGroupDirectoryPublicationEvidence>,
}

/// Coordinator-authenticated command advancing one bounded root publication step.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentPublicationRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub published_fleet_registry: FleetRegistryVersion,
    pub expected_published_component_count: u32,
}

///
/// RootComponentDirectorySynchronizationRequest
///
/// Coordinator-authenticated command advancing one affected-root scale-out Directory step.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentDirectorySynchronizationRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub source_fleet_registry: FleetRegistryVersion,
    pub published_fleet_registry: FleetRegistryVersion,
    pub expected_synchronized_component_count: u32,
}

///
/// RootComponentDirectorySynchronizationResponse
///
/// Compact exact progress for one root's affected existing service members.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentDirectorySynchronizationResponse {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub source_fleet_registry: FleetRegistryVersion,
    pub published_fleet_registry: FleetRegistryVersion,
    pub fleet_subnet_root: Principal,
    pub affected_component_count: u32,
    pub synchronized_component_count: u32,
    pub fleet_directory_content_hash: [u8; 32],
    pub complete: bool,
    pub synchronized_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
}

/// Coordinator-authenticated command advancing one bounded root activation step.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentActivationRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub expected_activated_component_count: u32,
    pub expected_root_runtime_active: bool,
}

/// Compact terminal evidence binding Component inventory and root runtime activation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootComponentActivationEvidence {
    pub fleet_activation_operation_id: [u8; 32],
    pub initial_inventory_hash: [u8; 32],
    pub component_count: u32,
    pub root_activated_at_ns: u64,
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
    pub published_component_count: u32,
    pub activated_component_count: u32,
    pub root_runtime_active: bool,
    pub result: Option<RootComponentProvisioningResult>,
    pub publication: Option<RootComponentPublicationEvidence>,
    pub activation: Option<RootComponentActivationEvidence>,
    pub accepted_at_ns: u64,
    pub provisioned_at_ns: Option<u64>,
    pub published_at_ns: Option<u64>,
    pub activation_started_at_ns: Option<u64>,
    pub runtimes_activated_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
}
