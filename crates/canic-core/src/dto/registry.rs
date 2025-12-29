use crate::{
    dto::{canister::CanisterEntryView, prelude::*},
    ids::CanisterRole,
};
use candid::{CandidType, Principal};

///
/// AppRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryView(pub Vec<(Principal, Principal)>);

///
/// SubnetRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryView(pub Vec<(CanisterRole, CanisterEntryView)>);
