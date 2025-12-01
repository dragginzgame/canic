use crate::{
    model::memory::{CanisterSummary, topology::SubnetCanisterChildren},
    types::Principal,
};
use candid::CandidType;
use serde::Serialize;

///
/// SubnetCanisterChildrenOps
///

pub struct SubnetCanisterChildrenOps;

impl SubnetCanisterChildrenOps {
    /// Return a paginated view of the canister's direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(offset: u64, limit: u64) -> SubnetCanisterChildrenPage {
        let all_children = SubnetCanisterChildren::export();
        let total = all_children.len() as u64;
        let start = offset.min(total) as usize;
        let end = offset.saturating_add(limit).min(total) as usize;
        let children = all_children[start..end].to_vec();

        SubnetCanisterChildrenPage {
            total,
            offset,
            limit,
            children,
        }
    }

    /// Lookup a child by principal
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<CanisterSummary> {
        SubnetCanisterChildren::find_by_pid(pid)
    }

    #[must_use]
    pub fn export() -> Vec<CanisterSummary> {
        SubnetCanisterChildren::export()
    }

    pub fn import(children: Vec<CanisterSummary>) {
        SubnetCanisterChildren::import(children);
    }
}

///
/// SubnetCanisterChildrenPage
/// Page of subnet canister children.
///

#[derive(CandidType, Serialize)]
pub struct SubnetCanisterChildrenPage {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub children: Vec<CanisterSummary>,
}
