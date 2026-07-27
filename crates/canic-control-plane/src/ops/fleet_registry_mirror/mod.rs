//! Module: ops::fleet_registry_mirror
//!
//! Responsibility: read and commit root-local Fleet Registry mirror state.
//! Does not own: snapshot validation, Coordinator calls, or lifecycle orchestration.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::fleet_registry_mirror::{
        RootFleetRegistryActiveRecord, RootFleetRegistryCandidateRecord,
        RootFleetRegistryMirrorStore,
    },
    view::fleet_registry_mirror::{
        RootFleetRegistryActiveView, RootFleetRegistryCandidateView, RootFleetRegistryMirrorView,
    },
};
use canic_core::dto::fleet_registry::{
    FleetDirectorySnapshot, FleetRegistrySnapshotResponse, FleetRegistryVersion,
    FleetSubnetRootSnapshotAcknowledgement,
};

///
/// FleetRegistryMirrorOps
///
/// Single-step root-local Fleet Registry mirror storage operations.
///

pub struct FleetRegistryMirrorOps;

impl FleetRegistryMirrorOps {
    pub(crate) fn current() -> RootFleetRegistryMirrorView {
        let data = RootFleetRegistryMirrorStore::export();
        RootFleetRegistryMirrorView {
            candidate: data.candidate.map(candidate_record_to_view),
            active: data.active.map(active_record_to_view),
        }
    }

    pub(crate) fn commit_candidate(
        snapshot: FleetRegistrySnapshotResponse,
        acknowledgement: Option<FleetSubnetRootSnapshotAcknowledgement>,
    ) {
        RootFleetRegistryMirrorStore::commit_candidate(RootFleetRegistryCandidateRecord {
            snapshot,
            acknowledgement,
        });
    }

    pub(crate) fn commit_active(
        previous_registry: FleetRegistryVersion,
        snapshot: FleetRegistrySnapshotResponse,
        directory: FleetDirectorySnapshot,
    ) {
        RootFleetRegistryMirrorStore::commit_active(RootFleetRegistryActiveRecord {
            previous_registry,
            snapshot,
            directory,
        });
    }
}

fn candidate_record_to_view(
    record: RootFleetRegistryCandidateRecord,
) -> RootFleetRegistryCandidateView {
    RootFleetRegistryCandidateView {
        snapshot: record.snapshot,
        acknowledgement: record.acknowledgement,
    }
}

fn active_record_to_view(record: RootFleetRegistryActiveRecord) -> RootFleetRegistryActiveView {
    RootFleetRegistryActiveView {
        previous_registry: record.previous_registry,
        snapshot: record.snapshot,
        directory: record.directory,
    }
}
