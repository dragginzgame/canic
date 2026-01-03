use crate::dto::{canister::CanisterEntryView, prelude::*};

///
/// AppRegistryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryEntryView {
    pub subnet_pid: Principal,
    pub root_pid: Principal,
}

///
/// AppRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryView(pub Vec<AppRegistryEntryView>);

///
/// SubnetRegistryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryEntryView {
    pub role: CanisterRole,
    pub entry: CanisterEntryView,
}

///
/// SubnetRegistryView
///

/// Note: the role duplicates `CanisterEntryView.role` so the entry view can
/// stay reusable across contexts.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryView(pub Vec<SubnetRegistryEntryView>);

///
/// DirectoryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectoryEntryView {
    pub role: CanisterRole,
    pub pid: Principal,
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
