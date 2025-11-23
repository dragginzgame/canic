use crate::{
    memory::{CanisterSummary, topology::SubnetCanisterRegistry},
    types::Principal,
};
use candid::CandidType;
use serde::Serialize;

///
/// CanisterChildrenOps
///

pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    /// Return a paginated view of the canister's direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(subnet_id: Principal, offset: u64, limit: u64) -> CanisterChildrenPage {
        let all_children = SubnetCanisterRegistry::children(subnet_id);
        let total = all_children.len() as u64;

        let start = offset.min(total) as usize;
        let end = offset.saturating_add(limit).min(total) as usize;
        let children = all_children[start..end].to_vec();

        CanisterChildrenPage {
            total,
            offset,
            limit,
            children,
        }
    }
}

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
