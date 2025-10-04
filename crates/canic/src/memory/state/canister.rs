use crate::{
    Error,
    cdk::structures::{Cell, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_unbounded,
    memory::{CanisterSummary, MemoryError, id::state::CANISTER_STATE_ID, state::StateError},
    types::CanisterType,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_STATE
//

eager_static! {
    static CANISTER_STATE: RefCell<Cell<CanisterStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(CanisterState, CANISTER_STATE_ID),
            CanisterStateData::default(),
        ));
}

///
/// CanisterStateError
///

#[derive(Debug, ThisError)]
pub enum CanisterStateError {
    #[error("canister summary has not been set")]
    CanisterNotSet,
}

impl From<CanisterStateError> for Error {
    fn from(err: CanisterStateError) -> Self {
        MemoryError::from(StateError::from(err)).into()
    }
}

///
/// CanisterStateData
/// Wraps the slimmed-down view of this canister.
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CanisterStateData {
    pub canister: Option<CanisterSummary>,
}

impl_storable_unbounded!(CanisterStateData);

///
/// CanisterState
///

pub struct CanisterState;

impl CanisterState {
    /// Get the current canister view (if any).
    #[must_use]
    pub fn get() -> Option<CanisterSummary> {
        CANISTER_STATE.with_borrow(|cell| cell.get().canister.clone())
    }

    /// Try to get the current canister summary, or error if missing.
    pub fn try_get_canister() -> Result<CanisterSummary, Error> {
        Self::get().ok_or_else(|| CanisterStateError::CanisterNotSet.into())
    }

    /// Set/replace the current canister summary.
    pub fn set_canister(canister: CanisterSummary) {
        CANISTER_STATE.with_borrow_mut(|cell| {
            cell.set(CanisterStateData {
                canister: Some(canister),
            })
        });
    }

    /// Check if this canister is root.
    #[must_use]
    pub fn is_root() -> bool {
        Self::get().is_some_and(|v| v.ty == CanisterType::ROOT)
    }

    /// Export current state (wraps the view).
    #[must_use]
    pub fn export() -> CanisterStateData {
        CANISTER_STATE.with_borrow(|cell| cell.get().clone())
    }

    /// Import state (replace existing).
    pub fn import(data: CanisterStateData) {
        CANISTER_STATE.with_borrow_mut(|cell| cell.set(data));
    }
}
