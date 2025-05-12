use crate::{
    Error,
    state::{
        StateError,
        core::{CHILD_INDEX, ChildIndex, ChildIndexData},
    },
};
use candid::Principal;

// get_data
#[must_use]
pub fn get_data() -> ChildIndexData {
    CHILD_INDEX.with_borrow(ChildIndex::get_data)
}

// get_canister
#[must_use]
pub fn get_canister(pid: &Principal) -> Option<String> {
    CHILD_INDEX.with_borrow(|this| this.get_canister(pid))
}

// try_get_canister
pub fn try_get_canister(pid: &Principal) -> Result<String, Error> {
    let path = CHILD_INDEX
        .with_borrow(|this| this.try_get_canister(pid))
        .map_err(StateError::ChildIndexError)?;

    Ok(path)
}

// insert_canister
pub fn insert_canister(pid: Principal, path: &str) {
    CHILD_INDEX.with_borrow_mut(|this| this.insert_canister(pid, path));
}
