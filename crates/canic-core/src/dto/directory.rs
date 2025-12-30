use crate::dto::prelude::*;

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
