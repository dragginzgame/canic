use crate::{
    Error,
    cdk::structures::{Cell, DefaultMemoryImpl, memory::VirtualMemory},
    icu_register_memory,
    memory::{CANISTER_ROOT_MEMORY_ID, MemoryError},
};
use candid::Principal;
use std::cell::RefCell;
use thiserror::Error as ThisError;

// Thread-local storage for root PID
thread_local! {
    static CANISTER_ROOT: RefCell<Cell<Option<Principal>, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            icu_register_memory!(CANISTER_ROOT_MEMORY_ID),
            None, // start empty
        ));
}

///
/// CanisterRootError
///

#[derive(Debug, ThisError)]
pub enum CanisterRootError {
    #[error("root pid has not been set")]
    NotSet,
}

///
/// CanisterRoot
///

pub struct CanisterRoot;

impl CanisterRoot {
    /// Get the root PID, if set.
    #[must_use]
    pub fn get() -> Option<Principal> {
        CANISTER_ROOT.with_borrow(|cell| *cell.get())
    }

    /// Try to get the root PID, or return an error if missing.
    pub fn try_get() -> Result<Principal, Error> {
        let pid = Self::get().ok_or_else(|| MemoryError::from(CanisterRootError::NotSet))?;

        Ok(pid)
    }

    /// Set the root PID (replace existing).
    pub fn set(pid: Principal) {
        CANISTER_ROOT.with_borrow_mut(|cell| cell.set(Some(pid)));
    }
}
