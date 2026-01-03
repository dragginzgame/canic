use crate::dto::{canister::CanisterEntryView, prelude::*};

///
/// AppRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AppRegistryView(pub Vec<(Principal, Principal)>);

///
/// SubnetRegistryView
///

/// Note: the tuple key duplicates `CanisterEntryView.role` so the entry view
/// can stay reusable across contexts.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryView(pub Vec<(CanisterRole, CanisterEntryView)>);

///
/// AppDirectoryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppDirectoryView(pub Vec<(CanisterRole, Principal)>);

///
/// SubnetDirectoryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetDirectoryView(pub Vec<(CanisterRole, Principal)>);
