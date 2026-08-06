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
    ids::ComponentDeploymentConfigurationDigest,
};

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
    pub accepted_at_ns: u64,
    pub receipt_content_hash: [u8; 32],
}
