use crate::{
    cdk::api::canister_self,
    model::memory::{
        CanisterSummary, Env,
        topology::{SubnetCanisterChildren, SubnetCanisterRegistry},
    },
};
use candid::CandidType;
use serde::Serialize;

///
/// SubnetCanisterChildrenOps
///

pub struct SubnetCanisterChildrenOps;

impl SubnetCanisterChildrenOps {
    #[must_use]
    pub fn fetch_children_from_topology() -> Vec<CanisterSummary> {
        if Env::is_root() {
            SubnetCanisterRegistry::children(canister_self())
        } else {
            SubnetCanisterChildren::export()
        }
    }

    /// Return a paginated view of the canister's direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(offset: u64, limit: u64) -> SubnetCanisterChildrenPage {
        let all_children = Self::fetch_children_from_topology();
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
