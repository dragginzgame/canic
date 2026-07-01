//! Module: cdk::types
//!
//! Responsibility: common IC-facing value types re-exported through Canic CDK.
//! Does not own: CDK API wrappers, stable structures, or serialization policy.
//! Boundary: centralizes type aliases and wrappers used by Canic-facing code.

pub mod cycles;
pub mod string;

pub use cycles::*;
pub use string::*;

use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, hash::Hash};

pub use candid::{Int, Nat, Principal};

pub type Subaccount = [u8; 32];

const DEFAULT_SUBACCOUNT: Subaccount = [0; 32];

#[derive(candid::CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

impl Account {
    #[must_use]
    pub fn effective_subaccount(&self) -> &Subaccount {
        self.subaccount.as_ref().unwrap_or(&DEFAULT_SUBACCOUNT)
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

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.effective_subaccount() == other.effective_subaccount()
    }
}

impl Eq for Account {}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

impl Hash for Account {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.effective_subaccount().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn account_treats_missing_subaccount_as_default() {
        let owner = principal(1);

        assert_eq!(
            Account {
                owner,
                subaccount: None
            },
            Account {
                owner,
                subaccount: Some([0; 32])
            }
        );
    }

    #[test]
    fn account_orders_by_effective_subaccount() {
        let owner = principal(1);
        let mut high = [0_u8; 32];
        high[31] = 1;

        assert!(
            Account {
                owner,
                subaccount: None
            } < Account {
                owner,
                subaccount: Some(high)
            }
        );
    }
}
