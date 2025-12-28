use crate::{dto::prelude::*, ids::CanisterRole};
use candid::Principal;

///
/// DirectoryView
/// Snapshot of a directory for sync / export
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectoryView(pub Vec<(CanisterRole, Principal)>);
