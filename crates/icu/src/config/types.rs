use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// Config
/// nothing here yet, but its coded so that's nice
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub controllers: Vec<Principal>,
}
