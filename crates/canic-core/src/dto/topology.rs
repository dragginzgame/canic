use crate::dto::{canister::CanisterInfo, prelude::*};

///
/// AppRegistryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryResponse(pub Vec<AppRegistryEntry>);

///
/// AppRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryEntry {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

///
/// SubnetRegistryResponse
///
/// External view of the subnet registry.
/// Each entry is identity-bearing (`pid`) and includes the full
/// canister record payload.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryResponse(pub Vec<SubnetRegistryEntry>);

///
/// SubnetRegistryEntry
///
/// Registry entry keyed by canister principal.
/// The `role` is duplicated outside the record for convenient
/// filtering and indexing by consumers.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryEntry {
    pub pid: Principal,
    pub role: CanisterRole,
    pub record: CanisterInfo,
}

///
/// AppDirectoryArgs
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppDirectoryArgs(pub Vec<DirectoryEntryInput>);

///
/// SubnetDirectoryArgs
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetDirectoryArgs(pub Vec<DirectoryEntryInput>);

///
/// DirectoryEntryInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectoryEntryInput {
    pub role: CanisterRole,
    pub pid: Principal,
}

///
/// DirectoryEntryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectoryEntryResponse {
    pub role: CanisterRole,
    pub pid: Principal,
}
