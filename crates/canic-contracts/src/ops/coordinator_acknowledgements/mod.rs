//! Lossless boundary projection; no acknowledgement or registry state is changed.
use crate::dto::{coordinator_acknowledgements as wire, fleet_registry as stored};

impl From<stored::FleetSubnetRootSnapshotAcknowledgement> for wire::RootAcknowledgement {
    fn from(value: stored::FleetSubnetRootSnapshotAcknowledgement) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            version: value.version.into(),
        }
    }
}
impl From<wire::RootAcknowledgement> for stored::FleetSubnetRootSnapshotAcknowledgement {
    fn from(value: wire::RootAcknowledgement) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            version: value.version.into(),
        }
    }
}

impl From<stored::FleetSubnetRootSnapshotAcknowledgementRequest> for wire::AcknowledgeRequest {
    fn from(value: stored::FleetSubnetRootSnapshotAcknowledgementRequest) -> Self {
        Self {
            version: value.version.into(),
        }
    }
}
impl From<wire::AcknowledgeRequest> for stored::FleetSubnetRootSnapshotAcknowledgementRequest {
    fn from(value: wire::AcknowledgeRequest) -> Self {
        Self {
            version: value.version.into(),
        }
    }
}
