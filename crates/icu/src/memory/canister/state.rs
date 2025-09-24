use crate::{
    Error,
    cdk::structures::{Cell, DefaultMemoryImpl, memory::VirtualMemory},
    icu_memory, impl_storable_unbounded,
    memory::{CANISTER_STATE_MEMORY_ID, CanisterView, MemoryError},
    types::CanisterType,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// CANISTER_STATE
thread_local! {
    static CANISTER_STATE: RefCell<Cell<CanisterStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            icu_memory!(CanisterState, CANISTER_STATE_MEMORY_ID),
            CanisterStateData::default(),
        ));
}

///
/// CanisterStateError
///

#[derive(Debug, ThisError)]
pub enum CanisterStateError {
    #[error("canister view has not been set")]
    ViewNotSet,
}

///
/// CanisterState
///

pub struct CanisterState;

impl CanisterState {
    /// Get the current canister view (if any).
    #[must_use]
    pub fn get_view() -> Option<CanisterView> {
        CANISTER_STATE.with_borrow(|cell| cell.get().view.clone())
    }

    /// Try to get the current view, or error if missing.
    pub fn try_get_view() -> Result<CanisterView, Error> {
        Self::get_view().ok_or_else(|| MemoryError::from(CanisterStateError::ViewNotSet).into())
    }

    /// Set/replace the current view.
    pub fn set_view(view: CanisterView) {
        CANISTER_STATE.with_borrow_mut(|cell| cell.set(CanisterStateData { view: Some(view) }));
    }

    /// Check if this canister is root.
    #[must_use]
    pub fn is_root() -> bool {
        Self::get_view().is_some_and(|v| v.ty == CanisterType::ROOT)
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

///
/// CanisterStateData
/// Wraps the slimmed-down view of this canister.
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CanisterStateData {
    pub view: Option<CanisterView>,
}

impl_storable_unbounded!(CanisterStateData);
