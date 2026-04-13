use crate::dto::{canister::CanisterInfo, prelude::*};

//
// AppRegistryResponse
//

#[derive(CandidType, Deserialize)]
pub struct AppRegistryResponse(pub Vec<AppRegistryEntry>);

//
// AppRegistryEntry
//

#[derive(CandidType, Deserialize)]
pub struct AppRegistryEntry {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

//
// SubnetRegistryResponse
//
// External subnet registry view.
//

#[derive(CandidType, Deserialize)]
pub struct SubnetRegistryResponse(pub Vec<SubnetRegistryEntry>);

//
// SubnetRegistryEntry
//
// Subnet registry entry.
//

#[derive(CandidType, Deserialize)]
pub struct SubnetRegistryEntry {
    pub pid: Principal,
    pub role: CanisterRole,
    pub record: CanisterInfo,
}

//
// AppIndexArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AppIndexArgs(pub Vec<IndexEntryInput>);

//
// SubnetIndexArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetIndexArgs(pub Vec<IndexEntryInput>);

//
// IndexEntryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct IndexEntryInput {
    pub role: CanisterRole,
    pub pid: Principal,
}

//
// IndexEntryResponse
//

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct IndexEntryResponse {
    pub role: CanisterRole,
    pub pid: Principal,
}
