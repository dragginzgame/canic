//! Module: storage::stable::fleet_registry_mirror
//!
//! Responsibility: own one root's private candidate or atomic active Registry/Directory evidence.
//! Does not own: snapshot validation, Coordinator calls, or lifecycle policy.
//! Boundary: mirror ops commit only fully validated exact evidence supplied by workflow.

use canic_core::dto::fleet_registry::{
    FleetDirectorySnapshot, FleetRegistrySnapshotResponse, FleetRegistryVersion,
    FleetSubnetRootSnapshotAcknowledgement,
};
#[cfg(feature = "root-control-plane")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, impl_storable_bounded,
    role_contract::allocation::memory::control_plane::ROOT_FLEET_REGISTRY_MIRROR_ID,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "root-control-plane")]
use std::cell::RefCell;

#[cfg(feature = "root-control-plane")]
const ROOT_FLEET_REGISTRY_MIRROR_MAX_BYTES: u32 = 4_194_304;

#[cfg(feature = "root-control-plane")]
struct RootFleetRegistryMirrorState;

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_FLEET_REGISTRY_MIRROR:
        RefCell<Cell<RootFleetRegistryMirrorStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.root_fleet_registry_mirror.v1",
                ty = RootFleetRegistryMirrorState,
                id = ROOT_FLEET_REGISTRY_MIRROR_ID
            ),
            RootFleetRegistryMirrorStateRecord::default(),
        ));
}

/// Durable validated candidate and optional exact Coordinator acknowledgement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetRegistryCandidateRecord {
    pub snapshot: FleetRegistrySnapshotResponse,
    pub acknowledgement: Option<FleetSubnetRootSnapshotAcknowledgement>,
}

/// Durable active mirror and its matching root-derived Fleet Directory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetRegistryActiveRecord {
    pub previous_registry: FleetRegistryVersion,
    pub snapshot: FleetRegistrySnapshotResponse,
    pub directory: FleetDirectorySnapshot,
}

/// Stable exclusive candidate-or-active wrapper.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetRegistryMirrorStateRecord {
    pub candidate: Option<RootFleetRegistryCandidateRecord>,
    pub active: Option<RootFleetRegistryActiveRecord>,
}

impl RootFleetRegistryMirrorStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootFleetRegistryMirrorStateRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootFleetRegistryMirrorStateRecord,
    ROOT_FLEET_REGISTRY_MIRROR_MAX_BYTES,
    false
);

/// Canonical export snapshot for root Registry mirror evidence.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootFleetRegistryMirrorData {
    pub candidate: Option<RootFleetRegistryCandidateRecord>,
    pub active: Option<RootFleetRegistryActiveRecord>,
}

impl RootFleetRegistryMirrorData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootFleetRegistryMirrorData";
}

/// Narrow stable-storage owner used by the root synchronization workflow.
pub struct RootFleetRegistryMirrorStore;

#[cfg(feature = "root-control-plane")]
impl RootFleetRegistryMirrorStore {
    pub(crate) fn export() -> RootFleetRegistryMirrorData {
        ROOT_FLEET_REGISTRY_MIRROR.with_borrow(|cell| RootFleetRegistryMirrorData {
            candidate: cell.get().candidate.clone(),
            active: cell.get().active.clone(),
        })
    }

    pub(crate) fn commit_candidate(record: RootFleetRegistryCandidateRecord) {
        ROOT_FLEET_REGISTRY_MIRROR.with_borrow_mut(|cell| {
            cell.set(RootFleetRegistryMirrorStateRecord {
                candidate: Some(record),
                active: None,
            });
        });
    }

    pub(crate) fn commit_active(record: RootFleetRegistryActiveRecord) {
        ROOT_FLEET_REGISTRY_MIRROR.with_borrow_mut(|cell| {
            cell.set(RootFleetRegistryMirrorStateRecord {
                candidate: None,
                active: Some(record),
            });
        });
    }
}
