use crate::{
    cdk::{api::canister_self, types::Principal},
    dto::Page,
    ids::CanisterRole,
    model::memory::{CanisterSummary, topology::SubnetCanisterChildren},
    ops::storage::{env::EnvOps, topology::SubnetCanisterRegistryOps},
    types::PageRequest,
};

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
    pub fn page(request: PageRequest) -> Page<CanisterSummary> {
        let request = request.clamped();
        let all_children = Self::resolve_children();
        let total = all_children.len() as u64;
        let start = request.offset.min(total) as usize;
        let end = request.offset.saturating_add(request.limit).min(total) as usize;
        let entries = all_children[start..end].to_vec();

        Page { entries, total }
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
    pub fn find_first_by_type(role: &CanisterRole) -> Option<CanisterSummary> {
        Self::resolve_children()
            .into_iter()
            .find(|child| &child.role == role)
    }

    #[must_use]
    pub(crate) fn export() -> Vec<CanisterSummary> {
        Self::resolve_children()
    }

    pub(crate) fn import(children: Vec<CanisterSummary>) {
        SubnetCanisterChildren::import(children);
    }
}
