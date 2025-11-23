use crate::memory::CanisterSummary;
use candid::CandidType;
use serde::Serialize;

///
/// CanisterChildrenPage
/// Page of subnet canister children.
///
#[derive(CandidType, Serialize)]
pub struct CanisterChildrenPage {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub children: Vec<CanisterSummary>,
}
