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
/// can stay reusable across contexts. This is intentional for now.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SubnetRegistryView(pub Vec<(CanisterRole, CanisterEntryView)>);
