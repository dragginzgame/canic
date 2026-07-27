//! Module: storage::stable::fleet_registry_mirror
//!
//! Responsibility: own one root's durable candidate Fleet Registry snapshot and acknowledgement.
//! Does not own: snapshot validation, Coordinator calls, Directory activation, or lifecycle policy.
//! Boundary: the root synchronization workflow commits only fully validated exact evidence.

use canic_core::dto::fleet_registry::{
    FleetRegistrySnapshotResponse, FleetSubnetRootSnapshotAcknowledgement,
};
#[cfg(feature = "root-control-plane")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, impl_storable_bounded,
    role_contract::allocation::memory::template::ROOT_FLEET_REGISTRY_MIRROR_ID,
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

impl RootFleetRegistryCandidateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootFleetRegistryCandidateRecord";
}

/// Stable optional wrapper before the first snapshot is staged.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFleetRegistryMirrorStateRecord {
    pub candidate: Option<RootFleetRegistryCandidateRecord>,
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
        })
    }

    pub(crate) fn commit(record: RootFleetRegistryCandidateRecord) {
        ROOT_FLEET_REGISTRY_MIRROR.with_borrow_mut(|cell| {
            cell.set(RootFleetRegistryMirrorStateRecord {
                candidate: Some(record),
            });
        });
    }
}
