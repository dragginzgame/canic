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
// AppDirectoryArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AppDirectoryArgs(pub Vec<DirectoryEntryInput>);

//
// SubnetDirectoryArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetDirectoryArgs(pub Vec<DirectoryEntryInput>);

//
// DirectoryEntryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DirectoryEntryInput {
    pub role: CanisterRole,
    pub pid: Principal,
}

//
// DirectoryEntryResponse
//

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct DirectoryEntryResponse {
    pub role: CanisterRole,
    pub pid: Principal,
}
