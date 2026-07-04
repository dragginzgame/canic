//! PocketIC upgrade-scenario helper for state migration regression tests.

use candid::Principal;
use ic_testkit::pic::Pic;
use std::{error::Error, fmt};

///
/// UpgradeScenario
///
/// Thin typed wrapper around one PocketIC canister upgrade scenario.
///

pub struct UpgradeScenario {
    pub pic: Pic,
    pub canister_id: Principal,
}

impl UpgradeScenario {
    #[must_use]
    pub const fn new(pic: Pic, canister_id: Principal) -> Self {
        Self { pic, canister_id }
    }

    #[must_use]
    pub fn install_old_wasm(&self, wasm: Vec<u8>, args: Vec<u8>) -> &Self {
        self.pic
            .install_canister(self.canister_id, wasm, args, None);
        self
    }

    #[must_use]
    pub fn seed_state<T>(&self, seed: impl FnOnce(&Pic, Principal) -> T) -> T {
        seed(&self.pic, self.canister_id)
    }

    pub fn upgrade_to_new_wasm(
        &self,
        wasm: Vec<u8>,
        args: Vec<u8>,
    ) -> Result<&Self, UpgradeScenarioError> {
        self.pic
            .upgrade_canister(self.canister_id, wasm, args, None)
            .map_err(|err| UpgradeScenarioError::Upgrade(err.to_string()))?;
        Ok(self)
    }

    #[must_use]
    pub fn query<T>(&self, query: impl FnOnce(&Pic, Principal) -> T) -> T {
        query(&self.pic, self.canister_id)
    }

    #[must_use]
    pub fn assert_invariants(&self, assert: impl FnOnce(&Pic, Principal)) -> &Self {
        assert(&self.pic, self.canister_id);
        self
    }
}

///
/// UpgradeScenarioError
///
/// Typed failure surfaced by the reusable upgrade scenario helper.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpgradeScenarioError {
    Upgrade(String),
}

impl fmt::Display for UpgradeScenarioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upgrade(message) => write!(formatter, "upgrade failed: {message}"),
        }
    }
}

impl Error for UpgradeScenarioError {}
