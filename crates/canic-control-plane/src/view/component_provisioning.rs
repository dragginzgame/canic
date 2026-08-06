//! Module: view::component_provisioning
//!
//! Responsibility: expose read-only root Component provisioning authority to workflows.
//! Does not own: persistence, validation, effects, or boundary serialization.
//! Boundary: ops constructs this view only from a validated durable aggregate record.

use canic_core::{
    dto::{
        component_provisioning::FleetSubnetRootProvisioningBatch,
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentGroupMemberPath,
        ComponentGroupPlacementId, ComponentSpecId,
    },
};

/// Read-only canonical reservation cursor for one accepted root batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningReservationCursorView {
    pub placement_index: u32,
    pub member_index: u32,
    pub reserved_component_count: u32,
}

/// Response-idempotent interpretation of one caller-bound expected cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentProvisioningReservationDisposition {
    Advance,
    Complete,
    Replay,
}

/// One exact accepted group member selected for the next reservation step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentMemberReservationView {
    pub member_operation_id: [u8; 32],
    pub group_placement: ComponentGroupPlacementId,
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
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
    pub accepted_at_ns: u64,
    pub receipt_content_hash: [u8; 32],
}
