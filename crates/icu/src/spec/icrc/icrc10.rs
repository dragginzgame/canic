use candid::CandidType;
use derive_more::Display;
use serde::Deserialize;

///
/// Icrc10Standard
///

#[derive(Clone, Copy, Debug, Display, Eq, Hash, PartialEq)]
pub enum Icrc10Standard {
    Icrc10,  // supported standards
    Icrc21,  // human readable representation of canister call
    Icrc103, // enhanced allowance query mechanism
}

///
/// Icrc10SupportedStandard
///

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct Icrc10SupportedStandard {
    pub url: String,
    pub name: String,
}
