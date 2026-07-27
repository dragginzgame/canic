//! Module: view::fleet_registry_mirror
//!
//! Responsibility: model read-only root-local Fleet Registry mirror projections.
//! Does not own: persisted records, validation, or state transitions.
//! Boundary: mirror ops construct these values for workflow consumption.

use canic_core::dto::fleet_registry::{
    FleetDirectorySnapshot, FleetRegistrySnapshotResponse, FleetRegistryVersion,
    FleetSubnetRootSnapshotAcknowledgement,
};

///
/// RootFleetRegistryCandidateView
///
/// Read-only staged Joining Registry evidence projected by mirror ops.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetRegistryCandidateView {
    pub snapshot: FleetRegistrySnapshotResponse,
    pub acknowledgement: Option<FleetSubnetRootSnapshotAcknowledgement>,
}

///
/// RootFleetRegistryActiveView
///
/// Read-only active Registry mirror and Fleet Directory projected by mirror ops.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFleetRegistryActiveView {
    pub previous_registry: FleetRegistryVersion,
    pub snapshot: FleetRegistrySnapshotResponse,
    pub directory: FleetDirectorySnapshot,
}

///
/// RootFleetRegistryMirrorView
///
/// Read-only exclusive candidate-or-active mirror projection.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootFleetRegistryMirrorView {
    pub candidate: Option<RootFleetRegistryCandidateView>,
    pub active: Option<RootFleetRegistryActiveView>,
}
