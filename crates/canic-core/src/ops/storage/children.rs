use crate::{
    cdk::{api::canister_self, types::Principal},
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
        snapshot::TopologyNodeView,
    },
    ids::CanisterRole,
    model::memory::children::{CanisterChildren, CanisterChildrenData},
    ops::{
        adapter::canister::{canister_summary_from_topology_node, canister_summary_to_view},
        env::EnvOps,
        storage::registry::SubnetRegistryOps,
    },
};

///
/// CanisterChildrenOps
///

pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    /// Resolve the authoritative internal children data.
    /// Root rebuilds from the registry; others use imported snapshot.
    fn resolve_children() -> CanisterChildrenData {
        if EnvOps::is_root() {
            SubnetRegistryOps::children(canister_self())
        } else {
            CanisterChildren::export()
        }
    }

    /// Return a paginated public view of direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(request: PageRequest) -> Page<CanisterSummaryView> {
        let request = request.clamped();
        let all = Self::resolve_children();

        let total = all.len() as u64;
        let start = request.offset.min(total) as usize;
        let end = request.offset.saturating_add(request.limit).min(total) as usize;

        let entries = all[start..end]
            .iter()
            .map(|(_, summary)| canister_summary_to_view(summary))
            .collect();

        Page { entries, total }
    }

    /// Lookup a child by principal (internal-only, identity based).
    #[must_use]
    pub(crate) fn find_by_pid(pid: &Principal) -> Option<CanisterSummaryView> {
        Self::resolve_children()
            .into_iter()
            .find(|(p, _)| p == pid)
            .map(|(_, summary)| canister_summary_to_view(&summary))
    }

    /// Lookup the first child of a given role.
    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<CanisterSummaryView> {
        Self::resolve_children()
            .into_iter()
            .find(|(_, summary)| &summary.role == role)
            .map(|(_, summary)| canister_summary_to_view(&summary))
    }

    /// Export identity-bearing children data (crate-private).
    #[must_use]
    pub(crate) fn export() -> CanisterChildrenData {
        Self::resolve_children()
    }

    /// Import identity-bearing children data (crate-private).
    pub(crate) fn import(data: CanisterChildrenData) {
        CanisterChildren::import(data);
    }

    /// Import identity-bearing children data from a topology snapshot view.
    pub(crate) fn import_view(children: Vec<TopologyNodeView>) {
        let data = children
            .into_iter()
            .map(|node| {
                let summary = canister_summary_from_topology_node(&node);
                (node.pid, summary)
            })
            .collect();
        CanisterChildren::import(data);
    }
}
