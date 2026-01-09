use crate::dto::{canister::CanisterRecordView, prelude::*};

///
/// AppRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryView(pub Vec<AppRegistryEntryView>);

///
/// AppRegistryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryEntryView {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

///
/// SubnetRegistryView
///
/// External view of the subnet registry.
/// Each entry is identity-bearing (`pid`) and includes the full
/// canister record payload.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryView(pub Vec<SubnetRegistryEntryView>);

///
/// SubnetRegistryEntryView
///
/// Registry entry keyed by canister principal.
/// The `role` is duplicated outside the record for convenient
/// filtering and indexing by consumers.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryEntryView {
    pub pid: Principal,
    pub role: CanisterRole,
    pub record: CanisterRecordView,
}

///
/// AppDirectoryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppDirectoryView(pub Vec<DirectoryEntryView>);

///
/// SubnetDirectoryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetDirectoryView(pub Vec<DirectoryEntryView>);

///
/// DirectoryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectoryEntryView {
    pub role: CanisterRole,
    pub pid: Principal,
}
