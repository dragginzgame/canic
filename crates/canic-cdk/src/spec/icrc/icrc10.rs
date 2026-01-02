use crate::spec::prelude::*;

///
/// Icrc10SupportedStandard
/// Response payload describing a single supported standard entry.
///

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct Icrc10SupportedStandard {
    pub url: String,
    pub name: String,
}
