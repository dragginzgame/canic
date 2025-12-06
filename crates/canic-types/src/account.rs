use crate::cdk::icrc_ledger_types::icrc1::account::Account as IcrcAccount;
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
/// [Account](https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md#value)
/// representation of ledgers supporting the ICRC-1 standard.
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
    #[inline]
    #[must_use]
    pub fn effective_subaccount(&self) -> &Subaccount {
        self.subaccount.as_ref().unwrap_or(DEFAULT_SUBACCOUNT)
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-1/TextualEncoding.md#textual-encoding-of-icrc-1-accounts
        match &self.subaccount {
            None => write!(f, "{}", self.owner),
            Some(subaccount) if subaccount == &[0; 32] => write!(f, "{}", self.owner),
            Some(subaccount) => {
                let checksum = full_account_checksum(self.owner.as_slice(), subaccount.as_slice());
                let hex_subaccount = hex::encode(subaccount.as_slice());
                let hex_subaccount = hex_subaccount.trim_start_matches('0');
                write!(f, "{}-{}.{}", self.owner, checksum, hex_subaccount)
            }
        }
    }
}

impl Eq for Account {}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.effective_subaccount() == other.effective_subaccount()
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

impl FromStr for Account {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let acc = IcrcAccount::from_str(s).map_err(|e| e.to_string())?;

        Ok(Self::new(acc.owner, acc.subaccount))
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

// make your internal code public, dfinity!
fn full_account_checksum(owner: &[u8], subaccount: &[u8]) -> String {
    let mut crc32hasher = crc32fast::Hasher::new();
    crc32hasher.update(owner);
    crc32hasher.update(subaccount);
    let checksum = crc32hasher.finalize().to_be_bytes();

    base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &checksum).to_lowercase()
}
