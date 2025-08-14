use candid::{CandidType, Principal};
use serde::Deserialize;
use std::collections::HashSet;

///
/// ConfigData
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigData {
    // controllers
    // a vec because we just append it to the controller arguments
    #[serde(default)]
    pub controllers: Vec<Principal>,

    #[serde(default)]
    pub whitelist: Option<WhiteList>,

    #[serde(default)]
    pub standards: Option<Standards>,
}

///
/// Whitelist
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhiteList {
    #[serde(default)]
    pub bypass_whitelist: bool,

    // principals
    // a hashset as we constantly have to do lookups
    #[serde(default)]
    pub principals: HashSet<Principal>,
}

///
/// Standards
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,
}
