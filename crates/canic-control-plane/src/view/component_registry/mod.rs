//! Module: view::component_registry
//!
//! Responsibility: model read-only root-local Component Registry meta authority.
//! Does not own: persisted records, validation, allocation, or lifecycle mutation.
//! Boundary: Component Registry ops construct these values for workflow consumption.

use canic_core::{
    dto::fleet_registry::FleetRegistryVersion,
    ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet},
};

///
/// RootComponentRegistryView
///
/// Read-only durable preparation authority and current allocation counters.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentRegistryView {
    pub root: FleetSubnetRootBinding,
    pub prepared_against_registry: FleetRegistryVersion,
    pub release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub encoded_bytes: u64,
}
