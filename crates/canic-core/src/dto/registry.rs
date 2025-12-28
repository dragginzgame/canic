use crate::{
    dto::{canister::CanisterEntryView, prelude::*},
    ids::CanisterRole,
};
use candid::{CandidType, Principal};

///
/// AppSubnetView
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub struct AppSubnetView {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

///
/// AppRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryView(pub Vec<(Principal, AppSubnetView)>);

///
/// SubnetRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryView(pub Vec<(CanisterRole, CanisterEntryView)>);
