//! Select one protected capacity before reconstructing receiver state.

use super::{
    ApplicationSessionStateError, LocalApplicationAuthorizationStateOps, invalidate_indexes,
};
use crate::model::auth::application_authorization::capacity::{
    DEFAULT_MAX_SESSIONS, ReceiverCapacity,
};
use std::cell::Cell;

thread_local! {
    static CAPACITY: Cell<ReceiverCapacity> = const {
        Cell::new(match ReceiverCapacity::new(DEFAULT_MAX_SESSIONS) {
            Some(capacity) => capacity,
            None => panic!("invalid default receiver capacity"),
        })
    };
}

pub(super) fn capacity() -> ReceiverCapacity {
    CAPACITY.with(Cell::get)
}

pub(super) fn select(max_sessions: u32) -> Result<(), ApplicationSessionStateError> {
    // Even rejected reconfiguration fences the old runtime and derived indexes.
    invalidate_indexes();
    let capacity =
        ReceiverCapacity::new(max_sessions).ok_or(ApplicationSessionStateError::InvalidCapacity)?;
    CAPACITY.with(|slot| slot.set(capacity));
    Ok(())
}

impl LocalApplicationAuthorizationStateOps {
    /// Select protected capacity before restoration. Never evicts retained rows;
    /// restoration refuses if the selected profile cannot contain the checkpoint.
    pub(crate) fn configure_receiver_capacity(
        max_sessions: u32,
    ) -> Result<(), ApplicationSessionStateError> {
        select(max_sessions)
    }
}

#[cfg(test)]
pub(super) fn restore_selection(capacity: ReceiverCapacity) {
    CAPACITY.with(|slot| slot.set(capacity));
}
