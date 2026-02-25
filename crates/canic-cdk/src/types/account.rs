use crate::icrc_ledger_types::icrc1::account::Account as Icrc1Account;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    str::FromStr,
};

///
/// Subaccount
///

pub type Subaccount = [u8; 32];

pub const DEFAULT_SUBACCOUNT: &Subaccount = &[0; 32];

///
/// Account
///
/// Code ported from icrc-ledger-types as we don't want to include that one, it's out of
/// date and has a lot of extra dependencies
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

impl Account {
    pub fn new<P: Into<Principal>, S: Into<Subaccount>>(owner: P, subaccount: Option<S>) -> Self {
        Self {
            owner: owner.into(),
            subaccount: subaccount.map(Into::into),
        }
    }

    /// The effective subaccount of an account - the subaccount if it is set, otherwise the default
    /// subaccount of all zeroes.
    #[must_use]
    pub fn effective_subaccount(&self) -> &Subaccount {
        self.subaccount.as_ref().unwrap_or(DEFAULT_SUBACCOUNT)
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let icrc = Icrc1Account::from(self);
        Display::fmt(&icrc, f)
    }
}

impl Eq for Account {}

impl FromStr for Account {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let acc = Icrc1Account::from_str(s).map_err(|e| e.to_string())?;

        Ok(Self::new(acc.owner, acc.subaccount))
    }
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.effective_subaccount() == other.effective_subaccount()
    }
}

impl From<Account> for Icrc1Account {
    fn from(a: Account) -> Self {
        Self {
            owner: a.owner,
            subaccount: a.subaccount,
        }
    }
}

impl From<&Account> for Icrc1Account {
    fn from(a: &Account) -> Self {
        Self {
            owner: a.owner,
            subaccount: a.subaccount,
        }
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

impl Hash for Account {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.effective_subaccount().hash(state);
    }
}

impl Ord for Account {
    fn cmp(&self, other: &Self) -> Ordering {
        self.owner.cmp(&other.owner).then_with(|| {
            self.effective_subaccount()
                .cmp(other.effective_subaccount())
        })
    }
}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
