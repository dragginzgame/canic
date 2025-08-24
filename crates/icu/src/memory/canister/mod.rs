pub mod children;
pub mod directory;
pub mod pool;
pub mod registry;
pub mod state;

use crate::{Error, ic::api::canister_self, memory::CanisterState, types::CanisterType};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// CanisterEntry
/// re-useable entry of type and principal
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
