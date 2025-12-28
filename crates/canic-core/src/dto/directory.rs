use crate::{dto::prelude::*, ids::CanisterRole};
use candid::Principal;

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
