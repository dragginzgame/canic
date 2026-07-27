//! Module: storage::stable::component_registry
//!
//! Responsibility: own one root's durable Component Registry preparation authority.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: Component Registry ops commit only complete authority validated by workflow.

#[cfg(feature = "root-control-plane")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    role_contract::allocation::memory::template::ROOT_COMPONENT_REGISTRY_META_ID,
};
use canic_core::{
    dto::fleet_registry::FleetRegistryVersion,
    ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet},
    impl_storable_bounded,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "root-control-plane")]
use std::cell::RefCell;

#[cfg(feature = "root-control-plane")]
const ROOT_COMPONENT_REGISTRY_STATE_MAX_BYTES: u32 = 65_536;

#[cfg(feature = "root-control-plane")]
struct RootComponentRegistryState;

#[cfg(feature = "root-control-plane")]
eager_static! {
    static ROOT_COMPONENT_REGISTRY:
        RefCell<Cell<RootComponentRegistryStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.root_component_registry.v1",
                ty = RootComponentRegistryState,
                id = ROOT_COMPONENT_REGISTRY_META_ID
            ),
            RootComponentRegistryStateRecord::default(),
        ));
}

///
/// RootComponentRegistryMetaRecord
///
/// Durable root authority and counters from which future Component allocations continue.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryMetaRecord {
    pub root: FleetSubnetRootBinding,
    pub prepared_against_registry: FleetRegistryVersion,
    pub release_set: FleetSubnetRootReleaseSet,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub encoded_bytes: u64,
}

///
/// RootComponentRegistryStateRecord
///
/// Stable optional wrapper before the exact root authority is prepared.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentRegistryStateRecord {
    pub current: Option<RootComponentRegistryMetaRecord>,
}

impl RootComponentRegistryStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentRegistryStateRecord";
}

#[cfg(feature = "root-control-plane")]
impl_storable_bounded!(
    RootComponentRegistryStateRecord,
    ROOT_COMPONENT_REGISTRY_STATE_MAX_BYTES,
    false
);

///
/// RootComponentRegistryData
///
/// Canonical export snapshot for root Component Registry meta authority.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootComponentRegistryData {
    pub current: Option<RootComponentRegistryMetaRecord>,
}

impl RootComponentRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentRegistryData";
}

///
/// RootComponentRegistryCommitOutcome
///
/// Result of preparing the one root-local Component Registry authority.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentRegistryCommitOutcome {
    Committed,
    Existing,
}

///
/// RootComponentRegistryCommitError
///
/// Rejection when preparation conflicts with already durable authority.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentRegistryCommitError {
    ConflictingState,
}

/// Narrow stable owner for root-local Component Registry meta authority.
pub struct RootComponentRegistryStore;

#[cfg(feature = "root-control-plane")]
impl RootComponentRegistryStore {
    pub(crate) fn prepare(
        record: RootComponentRegistryMetaRecord,
    ) -> Result<RootComponentRegistryCommitOutcome, RootComponentRegistryCommitError> {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => {
                    state.current = Some(record);
                    cell.set(state);
                    Ok(RootComponentRegistryCommitOutcome::Committed)
                }
                Some(existing) if existing == &record => {
                    Ok(RootComponentRegistryCommitOutcome::Existing)
                }
                Some(_) => Err(RootComponentRegistryCommitError::ConflictingState),
            }
        })
    }

    #[must_use]
    pub(crate) fn export() -> RootComponentRegistryData {
        ROOT_COMPONENT_REGISTRY.with_borrow(|cell| RootComponentRegistryData {
            current: cell.get().current.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn import(data: RootComponentRegistryData) {
        ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
            cell.set(RootComponentRegistryStateRecord {
                current: data.current,
            });
        });
    }
}
