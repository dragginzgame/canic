use crate::{
    Error,
    state::{
        StateError,
        core::{SUBNET_INDEX, SubnetIndex, SubnetIndexData},
    },
};
use candid::Principal;

// get_data
#[must_use]
pub fn get_data() -> SubnetIndexData {
    SUBNET_INDEX.with_borrow(SubnetIndex::get_data)
}

// set_data
pub fn set_data(data: SubnetIndexData) {
    SUBNET_INDEX.with_borrow_mut(|this| this.set_data(data));
}

// try_get_canister
pub fn try_get_canister(name: &str) -> Result<Principal, Error> {
    let canister_pid = SUBNET_INDEX
        .with_borrow(|this| this.try_get_canister(name))
        .map_err(StateError::SubnetIndexError)?;

    Ok(canister_pid)
}

// get_canister
#[must_use]
pub fn get_canister(name: &str) -> Option<Principal> {
    SUBNET_INDEX.with_borrow(|this| this.get_canister(name))
}

// set_canister
pub fn set_canister(name: &str, principal: Principal) {
    SUBNET_INDEX.with_borrow_mut(|this| this.set_canister(name.to_string(), principal));
}
