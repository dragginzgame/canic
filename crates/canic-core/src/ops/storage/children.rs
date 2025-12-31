use crate::{
    cdk::{api::canister_self, types::Principal},
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
        snapshot::TopologyChildView,
    },
    ids::CanisterRole,
    ops::{
        adapter::canister::{canister_summary_from_topology_child, canister_summary_to_view},
        runtime::env::EnvOps,
        storage::registry::subnet::SubnetRegistryOps,
        view::paginate::clamp_page_request,
    },
    storage::memory::children::{CanisterChildren, CanisterChildrenData},
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
            CanisterChildrenData {
                entries: SubnetRegistryOps::children(canister_self()),
            }
        } else {
            CanisterChildren::export()
        }
    }

    /// Return a paginated public view of direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(request: PageRequest) -> Page<CanisterSummaryView> {
        let request = clamp_page_request(request);
        let all = Self::resolve_children();

        let entries = &all.entries;
        let total = entries.len() as u64;
        let start = request.offset.min(total) as usize;
        let end = request.offset.saturating_add(request.limit).min(total) as usize;

        let entries = entries[start..end]
            .iter()
            .map(|(_, summary)| canister_summary_to_view(summary))
            .collect();

        Page { entries, total }
    }

    /// Lookup a child by principal (internal-only, identity based).
    #[must_use]
    pub(crate) fn find_by_pid(pid: &Principal) -> Option<CanisterSummaryView> {
        Self::resolve_children()
            .entries
            .into_iter()
            .find(|(p, _)| p == pid)
            .map(|(_, summary)| canister_summary_to_view(&summary))
    }

    /// Lookup the first child of a given role.
    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<CanisterSummaryView> {
        Self::resolve_children()
            .entries
            .into_iter()
            .find(|(_, summary)| &summary.role == role)
            .map(|(_, summary)| canister_summary_to_view(&summary))
    }

    /// Export child principals only (crate-private).
    #[must_use]
    pub(crate) fn pids() -> Vec<Principal> {
        Self::resolve_children()
            .entries
            .into_iter()
            .map(|(pid, _)| pid)
            .collect()
    }

    /// Import identity-bearing children data from a topology snapshot view.
    pub(crate) fn import_view(parent_pid: Principal, children: Vec<TopologyChildView>) {
        let entries = children
            .into_iter()
            .map(|node| {
                let summary = canister_summary_from_topology_child(&node, parent_pid);
                (node.pid, summary)
            })
            .collect();
        CanisterChildren::import(CanisterChildrenData { entries });
    }
}
