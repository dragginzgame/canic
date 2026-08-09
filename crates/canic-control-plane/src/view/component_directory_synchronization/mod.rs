//! Module: view::component_directory_synchronization
//!
//! Responsibility: expose read-only root scale-out Directory synchronization authority.
//! Does not own: persistence, Registry mutation, target calls, or endpoint DTOs.
//! Boundary: ops projects validated durable records for workflow orchestration.

use candid::Principal;
use canic_core::{
    dto::{
        component_provisioning::RootComponentDirectorySynchronizationResponse,
        component_registry::ComponentRegistryHead, fleet_registry::FleetRegistryVersion,
    },
    ids::ComponentInstanceId,
};

/// Existing active service member selected before Fleet Registry mirror advancement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDirectorySynchronizationTargetView {
    pub component: ComponentInstanceId,
    pub canister_id: Principal,
    pub allocation_operation_id: [u8; 32],
    pub source_registry: ComponentRegistryHead,
}

/// Durable pre-call intent for one exact active Component Directory replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDirectorySynchronizationIntentView {
    pub component_index: u32,
    pub component: ComponentInstanceId,
    pub canister_id: Principal,
    pub allocation_operation_id: [u8; 32],
    pub previous_registry: ComponentRegistryHead,
    pub registry: ComponentRegistryHead,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub started_at_ns: u64,
}

/// Validated durable progress for one affected root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentDirectorySynchronizationView {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub source_fleet_registry: FleetRegistryVersion,
    pub published_fleet_registry: FleetRegistryVersion,
    pub fleet_subnet_root: Principal,
    pub fleet_directory_content_hash: [u8; 32],
    pub targets: Vec<RootComponentDirectorySynchronizationTargetView>,
    pub synchronized_component_count: u32,
    pub in_flight: Option<RootComponentDirectorySynchronizationIntentView>,
    pub planned_at_ns: u64,
    pub synchronized_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
    pub complete: bool,
}

/// Response-idempotent next action for one root synchronization command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootComponentDirectorySynchronizationDisposition {
    Current(Box<RootComponentDirectorySynchronizationResponse>),
    Invoke(RootComponentDirectorySynchronizationIntentView),
    Reconcile(RootComponentDirectorySynchronizationIntentView),
}
