pub mod children;
pub mod directory;
pub mod pool;
pub mod registry;
pub mod state;

use crate::{Error, canister::CanisterType, ic::api::canister_self, memory::CanisterState};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// CanisterEntry
/// re-useable entry of type and principal
///

#[derive(CandidType, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CanisterEntry {
    pub canister_type: CanisterType,
    pub principal: Principal,
}

impl CanisterEntry {
    pub fn this() -> Result<Self, Error> {
        let canister_type = CanisterState::try_get_type()?;

        Ok(Self {
            canister_type,
            principal: canister_self(),
        })
    }
}
