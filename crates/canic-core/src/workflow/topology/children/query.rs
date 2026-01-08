use crate::{
    dto::{
        canister::{CanisterChildView, CanisterSummaryView},
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    ops::{
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{
            children::{CanisterChildrenOps, ChildrenSnapshot},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{topology::children::mapper::ChildrenMapper, view::paginate::paginate_vec},
};

///
/// CanisterChildrenQuery
///

pub struct CanisterChildrenQuery;

impl CanisterChildrenQuery {
    pub fn page(page: PageRequest) -> Page<CanisterSummaryView> {
        let views = ChildrenMapper::snapshot_to_views(Self::snapshot());

        // 3. Paginate in workflow
        paginate_vec(views, page)
    }

    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<CanisterChildView> {
        Self::snapshot()
            .entries
            .into_iter()
            .find(|entry| &entry.role == role)
            .map(ChildrenMapper::child_snapshot_to_child_view)
    }

    fn snapshot() -> ChildrenSnapshot {
        if EnvOps::is_root() {
            // Root derives children from the registry (not the local cache).
            let snapshot = SubnetRegistryOps::snapshot();
            ChildrenMapper::from_registry_snapshot(&snapshot, IcOps::canister_self())
        } else {
            // Non-root uses the cached children populated by topology cascade.
            CanisterChildrenOps::snapshot()
        }
    }
}
