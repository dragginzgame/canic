use crate::{
    cdk::{api::canister_self, types::Principal},
    ids::CanisterRole,
    model::memory::{CanisterSummary, topology::SubnetCanisterChildren},
    ops::model::memory::{env::EnvOps, topology::SubnetCanisterRegistryOps},
    types::PageRequest,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// SubnetCanisterChildrenOps
///

pub struct SubnetCanisterChildrenOps;

impl SubnetCanisterChildrenOps {
    /// Resolve the canonical view of direct children for the current canister.
    /// Root rebuilds from the registry; children rely on their imported snapshot.
    fn resolve_children() -> Vec<CanisterSummary> {
        if EnvOps::is_root() {
            SubnetCanisterRegistryOps::children(canister_self())
        } else {
            SubnetCanisterChildren::export()
        }
    }

    /// Return a paginated view of the canister's direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(request: PageRequest) -> SubnetCanisterChildrenPage {
        let request = request.clamped();
        let all_children = Self::resolve_children();
        let total = all_children.len() as u64;
        let start = request.offset.min(total) as usize;
        let end = request.offset.saturating_add(request.limit).min(total) as usize;
        let children = all_children[start..end].to_vec();

        SubnetCanisterChildrenPage { total, children }
    }

    /// Lookup a child by principal
    #[must_use]
    pub(crate) fn find_by_pid(pid: &Principal) -> Option<CanisterSummary> {
        Self::resolve_children()
            .into_iter()
            .find(|child| child.pid == *pid)
    }

    /// Lookup the first child of a given type
    #[must_use]
    pub fn find_first_by_type(ty: &CanisterRole) -> Option<CanisterSummary> {
        Self::resolve_children()
            .into_iter()
            .find(|child| &child.ty == ty)
    }

    #[must_use]
    pub(crate) fn export() -> Vec<CanisterSummary> {
        Self::resolve_children()
    }

    pub(crate) fn import(children: Vec<CanisterSummary>) {
        SubnetCanisterChildren::import(children);
    }
}

///
/// SubnetCanisterChildrenPage
/// Page of subnet canister children.
///

#[derive(CandidType, Debug, Deserialize, Serialize)]
pub struct SubnetCanisterChildrenPage {
    pub total: u64,
    pub children: Vec<CanisterSummary>,
}
