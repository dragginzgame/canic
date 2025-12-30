use crate::dto::prelude::*;

///
/// CanisterEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterEntryView {
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

///
/// CanisterSummaryView
/// Minimal view for children/subnet directories
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterSummaryView {
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
