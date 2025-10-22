use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// Account
///

pub type Subaccount = [u8; 32];

pub const DEFAULT_SUBACCOUNT: &Subaccount = &[0; 32];

/// [Account](https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md#value)
/// representation of ledgers supporting the ICRC-1 standard.
///
#[derive(Serialize, CandidType, Deserialize, Clone, Eq, PartialEq, Debug, Copy)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

impl Account {
    /// The effective subaccount of an account - the subaccount if it is set, otherwise the default
    /// subaccount of all zeroes.
    #[inline]
    #[must_use]
    pub fn effective_subaccount(&self) -> &Subaccount {
        self.subaccount.as_ref().unwrap_or(DEFAULT_SUBACCOUNT)
    }
}

impl From<Principal> for Account {
    fn from(owner: Principal) -> Self {
        Self {
            owner,
            subaccount: None,
        }
    }
}
