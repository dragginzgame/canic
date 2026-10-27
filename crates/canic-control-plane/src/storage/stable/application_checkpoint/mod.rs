//! Persist Root checkpoint ownership outside the participant snapshots.
//!
//! Ops validates complete records before committing; this store performs no IC calls.

use canic_core::{
    cdk::structures::{DefaultMemoryImpl, StableCell},
    control_plane_support::model::application_checkpoint::RootCheckpointRecord,
    impl_storable_bounded,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/// Bounded Root checkpoint state; absence is valid only before initial issuance.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootCheckpointStateRecord {
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub current: Option<RootCheckpointRecord>,
}

impl_storable_bounded!(RootCheckpointStateRecord, 8_388_608, false);

thread_local! {
    static STATE: RefCell<StableCell<RootCheckpointStateRecord, DefaultMemoryImpl>> =
        RefCell::new(StableCell::init(
            canic_core::memory::open(crate::memory::keys::ROOT_APPLICATION_CHECKPOINT_V1),
            RootCheckpointStateRecord::default(),
        ));
}

/// One stable owner for the complete original unit and its acknowledgements.
pub struct RootCheckpointStore;

impl RootCheckpointStore {
    pub(crate) fn read() -> Option<RootCheckpointRecord> {
        STATE.with_borrow(|state| state.get().current.clone())
    }

    pub(crate) fn write(
        record: RootCheckpointRecord,
    ) -> Result<(), canic_core::control_plane_support::error::InternalError> {
        let state = RootCheckpointStateRecord {
            current: Some(record),
        };
        let bytes = canic_contracts::serialization::serialize(&state)
            .map_err(|_| canic_core::control_plane_support::error::InternalError::invariant())?;
        if bytes.len() > 8_388_608 {
            return Err(
                canic_core::control_plane_support::error::InternalError::resource_exhausted(),
            );
        }
        let state_record = state;
        STATE.with_borrow_mut(|state| state.set(state_record));
        Ok(())
    }
}
