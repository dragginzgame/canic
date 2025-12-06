use crate::spec::prelude::*;
use derive_more::Display;

///
/// Icrc10Standard
/// Enumeration of well-known ICRC-10 standards with descriptive variants.
///

#[derive(Clone, Copy, Debug, Display, Eq, Hash, PartialEq)]
pub enum Icrc10Standard {
    Icrc10,  // supported standards
    Icrc21,  // human readable representation of canister call
    Icrc103, // enhanced allowance query mechanism
}

///
/// Icrc10SupportedStandard
/// Response payload describing a single supported standard entry.
///

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct Icrc10SupportedStandard {
    pub url: String,
    pub name: String,
}
