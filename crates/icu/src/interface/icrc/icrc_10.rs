use candid::CandidType;
use derive_more::Display;
use serde::Deserialize;

///
/// Icrc10Standard
///

#[derive(Debug, Display, Eq, PartialEq, Hash, Clone)]
pub enum Icrc10Standard {
    Icrc10,
    Icrc21,
}

///
/// Icrc10SupportedStandard
///

#[derive(CandidType, Deserialize, Eq, PartialEq, Debug)]
pub struct Icrc10SupportedStandard {
    pub url: String,
    pub name: String,
}
