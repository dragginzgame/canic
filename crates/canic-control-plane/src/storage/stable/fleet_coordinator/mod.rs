//! Module: storage::stable::fleet_coordinator
//!
//! Responsibility: own the Fleet Coordinator's authoritative stable Registry record.
//! Does not own: Registry validation, endpoint authorization, or lifecycle orchestration.
//! Boundary: Coordinator ops may commit or export one complete validated record.

#[cfg(feature = "fleet-coordinator-canister")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, impl_storable_bounded,
    role_contract::allocation::memory::template::FLEET_COORDINATOR_REGISTRY_ID,
};
use canic_core::{
    control_plane_support::config::ComponentTopology,
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryVersion, FleetSubnetRootEntry,
        FleetSubnetRootSnapshotAcknowledgement,
    },
    ids::{AppId, FleetRegistryAuthority},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "fleet-coordinator-canister")]
use std::cell::RefCell;

#[cfg(feature = "fleet-coordinator-canister")]
// The record may contain one topology, one Registry snapshot, and the
// root-entry portion of that Registry again as immutable join receipts, plus
// one exact acknowledgement per current root.
const FLEET_COORDINATOR_STATE_MAX_BYTES: u32 = 8_388_608;

#[cfg(feature = "fleet-coordinator-canister")]
struct FleetCoordinatorRegistryState;

#[cfg(feature = "fleet-coordinator-canister")]
eager_static! {
    static FLEET_COORDINATOR_STATE:
        RefCell<Cell<FleetCoordinatorStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.fleet_coordinator_registry.v1",
                ty = FleetCoordinatorRegistryState,
                id = FLEET_COORDINATOR_REGISTRY_ID
            ),
            FleetCoordinatorStateRecord::default(),
        ));
}

///
/// FleetCoordinatorRegistryRecord
///
/// Complete protected topology and current canonical Fleet Registry.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetCoordinatorRegistryRecord {
    pub configured_app: AppId,
    pub authority: FleetRegistryAuthority,
    pub component_topology: ComponentTopology,
    pub registry: FleetRegistry,
    pub root_join_receipts: Vec<FleetSubnetRootJoinReceiptRecord>,
    pub root_snapshot_acknowledgements: Vec<FleetSubnetRootSnapshotAcknowledgement>,
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
impl FleetCoordinatorRegistryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetCoordinatorRegistryRecord";
}

///
/// FleetSubnetRootJoinReceiptRecord
///
/// Persisted exact response authority for one root's original `Joining` commit.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootJoinReceiptRecord {
    pub entry: FleetSubnetRootEntry,
    pub version: FleetRegistryVersion,
}

///
/// FleetCoordinatorStateRecord
///
/// Stable optional state wrapper used before fresh Coordinator initialization.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg(feature = "fleet-coordinator-canister")]
pub struct FleetCoordinatorStateRecord {
    pub current: Option<FleetCoordinatorRegistryRecord>,
}

#[cfg(feature = "fleet-coordinator-canister")]
impl_storable_bounded!(
    FleetCoordinatorStateRecord,
    FLEET_COORDINATOR_STATE_MAX_BYTES,
    false
);

///
/// FleetCoordinatorRegistryData
///
/// Canonical export snapshot for the Fleet Coordinator Registry allocation.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetCoordinatorRegistryData {
    pub current: Option<FleetCoordinatorRegistryRecord>,
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
impl FleetCoordinatorRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetCoordinatorRegistryData";
}

///
/// FleetCoordinatorCommitOutcome
///
/// Result of committing fresh genesis state to the single Coordinator cell.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "fleet-coordinator-canister")]
pub enum FleetCoordinatorCommitOutcome {
    Committed,
    Existing,
}

///
/// FleetCoordinatorCommitError
///
/// Stable-store rejection when fresh genesis conflicts with existing state.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "fleet-coordinator-canister")]
pub enum FleetCoordinatorCommitError {
    ConflictingState,
    Uninitialized,
}

///
/// FleetCoordinatorRegistryStore
///
/// Narrow stable-storage owner used only by Coordinator ops.
///

#[cfg(feature = "fleet-coordinator-canister")]
pub struct FleetCoordinatorRegistryStore;

#[cfg(feature = "fleet-coordinator-canister")]
impl FleetCoordinatorRegistryStore {
    pub(crate) fn commit_genesis(
        record: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, FleetCoordinatorCommitError> {
        FLEET_COORDINATOR_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => {
                    state.current = Some(record);
                    cell.set(state);
                    Ok(FleetCoordinatorCommitOutcome::Committed)
                }
                Some(existing) if existing == &record => {
                    Ok(FleetCoordinatorCommitOutcome::Existing)
                }
                Some(_) => Err(FleetCoordinatorCommitError::ConflictingState),
            }
        })
    }

    pub(crate) fn commit_transition(
        expected: &FleetCoordinatorRegistryRecord,
        next: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, FleetCoordinatorCommitError> {
        FLEET_COORDINATOR_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => Err(FleetCoordinatorCommitError::Uninitialized),
                Some(existing) if existing == &next => Ok(FleetCoordinatorCommitOutcome::Existing),
                Some(existing) if existing != expected => {
                    Err(FleetCoordinatorCommitError::ConflictingState)
                }
                Some(_) => {
                    state.current = Some(next);
                    cell.set(state);
                    Ok(FleetCoordinatorCommitOutcome::Committed)
                }
            }
        })
    }

    #[must_use]
    pub(crate) fn export() -> FleetCoordinatorRegistryData {
        FLEET_COORDINATOR_STATE.with_borrow(|cell| FleetCoordinatorRegistryData {
            current: cell.get().current.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn import(data: FleetCoordinatorRegistryData) {
        FLEET_COORDINATOR_STATE.with_borrow_mut(|cell| {
            cell.set(FleetCoordinatorStateRecord {
                current: data.current,
            });
        });
    }
}
