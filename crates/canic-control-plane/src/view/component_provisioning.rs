//! Module: view::component_provisioning
//!
//! Responsibility: expose read-only root Component provisioning authority to workflows.
//! Does not own: persistence, validation, effects, or boundary serialization.
//! Boundary: ops constructs this view only from a validated durable aggregate record.

use canic_core::{
    dto::{
        component_deployment::{
            ComponentDeploymentLabel, ComponentDeploymentLimits, ComponentDeploymentPurpose,
            ProtectedComponentDeployment,
        },
        component_provisioning::FleetSubnetRootProvisioningBatch,
        component_provisioning::{
            ComponentGroupDirectory, RootComponentActivationEvidence,
            RootComponentProvisioningPhase, RootComponentProvisioningResult,
            RootComponentPublicationEvidence,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupMemberPath,
        ComponentGroupPlacementId, ComponentGroupSpecId, ComponentSpecId,
    },
};

/// Read-only canonical reservation cursor for one accepted root batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningReservationCursorView {
    pub placement_index: u32,
    pub member_index: u32,
    pub reserved_component_count: u32,
}

/// Read-only canonical pool-claim cursor for one accepted root batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningClaimCursorView {
    pub placement_index: u32,
    pub member_index: u32,
    pub claimed_component_count: u32,
}

/// Read-only canonical install cursor for one accepted root batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningInstallCursorView {
    pub placement_index: u32,
    pub member_index: u32,
    pub installed_component_count: u32,
}

/// Read-only canonical Registry-commit cursor for one accepted root batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningRegistryCursorView {
    pub placement_index: u32,
    pub member_index: u32,
    pub registry_committed_component_count: u32,
}

/// Response-idempotent interpretation of one caller-bound expected cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentProvisioningAdvanceDisposition {
    Advance,
    Complete,
    Replay,
}

/// One exact accepted group member selected for the next bounded provisioning step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningMemberView {
    pub member_operation_id: [u8; 32],
    pub group_placement: ComponentGroupPlacementId,
    pub component_group: ComponentGroupSpecId,
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub purpose: ComponentDeploymentPurpose,
    pub labels: Vec<ComponentDeploymentLabel>,
    pub limits: ComponentDeploymentLimits,
}

/// Read-only accepted root batch and its exact replay receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningView {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub batch: FleetSubnetRootProvisioningBatch,
    pub placement_count: u32,
    pub component_count: u32,
    pub reservation_cursor: RootComponentProvisioningReservationCursorView,
    pub claim_cursor: RootComponentProvisioningClaimCursorView,
    pub install_cursor: RootComponentProvisioningInstallCursorView,
    pub registry_cursor: RootComponentProvisioningRegistryCursorView,
    pub phase: RootComponentProvisioningPhase,
    pub result: Option<RootComponentProvisioningResult>,
    pub publication: Option<RootComponentPublicationEvidence>,
    pub published_component_count: u32,
    pub activated_component_count: u32,
    pub root_runtime_active: bool,
    pub publication_in_flight: Option<RootComponentPublicationIntentView>,
    pub activation: Option<RootComponentActivationEvidence>,
    pub accepted_at_ns: u64,
    pub provisioned_at_ns: Option<u64>,
    pub publication_started_at_ns: Option<u64>,
    pub published_at_ns: Option<u64>,
    pub activation_started_at_ns: Option<u64>,
    pub runtimes_activated_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
}

/// Read-only pre-call intent for one exact Component Directory delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentPublicationIntentView {
    pub component_index: u32,
    pub canister_id: candid::Principal,
    pub directory_authority_hash: [u8; 32],
    pub started_at_ns: u64,
}

/// One exact next prepared Component and its immutable group projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentPublicationMemberView {
    pub component_index: u32,
    pub member_operation_id: [u8; 32],
    pub binding: ComponentBinding,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
    pub deployment: ProtectedComponentDeployment,
    pub component_group: ComponentGroupDirectory,
}

/// Exact retained runtime authority for one provisioned Component Group member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentGroupRuntimeAuthorityView {
    pub deployment: ProtectedComponentDeployment,
    pub component_group: ComponentGroupDirectory,
}
