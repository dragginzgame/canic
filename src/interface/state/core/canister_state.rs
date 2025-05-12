use crate::{
    Error,
    state::{
        StateError,
        core::{CANISTER_STATE, CanisterState, CanisterStateData},
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
        .with_borrow(|state| state.get_path())
        .map_err(StateError::CanisterStateError)?;

    Ok(())
}

// set_path
pub fn set_path(path: &str) -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow_mut(|state| state.set_path(path))
        .map_err(StateError::CanisterStateError)?;

    Ok(())
}

// get_root_pid
pub fn get_root_pid() -> Result<Principal, Error> {
    let root_id = CANISTER_STATE
        .with_borrow(CanisterState::get_root_pid)
        .map_err(StateError::CanisterStateError)?;

    Ok(root_id)
}

// set_root_id
pub fn set_root_id(pid: Principal) -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow_mut(|state| state.set_root_id(pid))
        .map_err(StateError::CanisterStateError)?;

    Ok(())
}

// get_parent_pid
#[must_use]
pub fn get_parent_pid() -> Option<Principal> {
    CANISTER_STATE.with_borrow(CanisterState::get_parent_pid)
}

// set_parent_id
pub fn set_parent_id(pid: Principal) -> Result<(), Error> {
    CANISTER_STATE
        .with_borrow_mut(|state| state.set_parent_id(pid))
        .map_err(StateError::CanisterStateError)?;

    Ok(())
}
