use candid::Principal;
use serde::Deserialize;
use std::collections::HashSet;

///
/// ConfigData
///

#[derive(Clone, Debug, Default, Deserialize)]
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

    #[serde(default)]
    pub cycle_tracker: bool,
}

///
/// Whitelist
///

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhiteList {
    // principals
    // a hashset as we constantly have to do lookups
    // strings because then we can validate and know if there are any bad ones
    #[serde(default)]
    pub principals: HashSet<String>,
}

///
/// Standards
///

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,
}
