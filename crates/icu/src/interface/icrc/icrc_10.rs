use candid::CandidType;
use serde::Deserialize;

///
/// Icrc10SupportedStandard
///

#[derive(CandidType, Deserialize, Eq, PartialEq, Debug)]
pub struct Icrc10SupportedStandard {
    pub url: String,
    pub name: String,
}
