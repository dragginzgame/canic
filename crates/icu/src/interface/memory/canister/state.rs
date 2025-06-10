use crate::{
    Error,
    memory::{
        CANISTER_STATE, MemoryError,
        canister::{CanisterState, CanisterStateData},
    },
};
use candid::Principal;

// get_data
#[must_use]
pub fn get_data() -> CanisterStateData {
    CANISTER_STATE.with_borrow(CanisterState::get_data)
}

// get_path
pub fn get_path() -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow(CanisterState::get_path)
        .map_err(MemoryError::CanisterStateError)?;

    Ok(())
}

// set_path
pub fn set_path(path: &str) -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow_mut(|state| state.set_path(path))
        .map_err(MemoryError::CanisterStateError)?;

    Ok(())
}

// get_root_pid
pub fn get_root_pid() -> Result<Principal, Error> {
    let root_id = CANISTER_STATE
        .with_borrow(CanisterState::get_root_pid)
        .map_err(MemoryError::CanisterStateError)?;

    Ok(root_id)
}

// set_root_pid
pub fn set_root_pid(pid: Principal) -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow_mut(|state| state.set_root_pid(pid))
        .map_err(MemoryError::CanisterStateError)?;

    Ok(())
}

// get_parent_pid
#[must_use]
pub fn get_parent_pid() -> Option<Principal> {
    CANISTER_STATE.with_borrow(CanisterState::get_parent_pid)
}

// set_parent_pid
pub fn set_parent_pid(pid: Principal) -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow_mut(|state| state.set_parent_pid(pid))
        .map_err(MemoryError::CanisterStateError)?;

    Ok(())
}
