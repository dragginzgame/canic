use crate::{
    Error,
    cdk::structures::{Cell, DefaultMemoryImpl, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_STATE_MEMORY_ID, CanisterEntry, MemoryError},
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// thread local
thread_local! {
    static CANISTER_STATE: RefCell<Cell<CanisterStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            icu_register_memory!(CANISTER_STATE_MEMORY_ID),
            CanisterStateData::default(), // start empty
        ));
}

//
// CanisterStateError
//

#[derive(Debug, ThisError)]
pub enum CanisterStateError {
    #[error("canister entry has not been set")]
    EntryNotSet,

    #[error("root pid has not been set")]
    RootNotSet,
}

//
// API
//

pub struct CanisterState;

impl CanisterState {
    /// Get the current entry
    #[must_use]
    pub fn get_entry() -> Option<CanisterEntry> {
        CANISTER_STATE.with_borrow(|cell| cell.get().entry.clone())
    }

    /// Try to get the current entry, or error if missing
    pub fn try_get_entry() -> Result<CanisterEntry, Error> {
        Self::get_entry().ok_or_else(|| MemoryError::from(CanisterStateError::EntryNotSet).into())
    }

    /// Set the current entry (replace existing)
    pub fn set_entry(entry: CanisterEntry) {
        CANISTER_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            state.entry = Some(entry);
            cell.set(state);
        });
    }

    /// Check if this canister is the root
    #[must_use]
    pub fn is_root() -> bool {
        Self::get_entry().is_some_and(|e| e.ty == CanisterType::ROOT)
    }

    /// Export current state
    #[must_use]
    pub fn export() -> CanisterStateData {
        CANISTER_STATE.with_borrow(|cell| cell.get().clone())
    }

    /// Import state (replace existing)
    pub fn import(data: CanisterStateData) {
        CANISTER_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    /// Set root_pid
    pub fn set_root_pid(pid: Principal) {
        CANISTER_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            state.root_pid = Some(pid);
            cell.set(state);
        });
    }

    /// Get root_pid
    #[must_use]
    pub fn get_root_pid() -> Option<Principal> {
        CANISTER_STATE.with_borrow(|cell| cell.get().root_pid)
    }

    /// Try to get root_pid
    pub fn try_get_root_pid() -> Result<Principal, Error> {
        Self::get_root_pid().ok_or_else(|| MemoryError::from(CanisterStateError::RootNotSet).into())
    }
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CanisterStateData {
    pub entry: Option<CanisterEntry>,
    pub root_pid: Option<Principal>,
}

impl_storable_unbounded!(CanisterStateData);
