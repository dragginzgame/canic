use crate::{
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    ops::{
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{topology::children::mapper::ChildrenMapper, view::paginate::paginate_vec},
};

///
/// CanisterChildrenQuery
///

pub struct CanisterChildrenQuery;

impl CanisterChildrenQuery {
    pub fn page(page: PageRequest) -> Page<CanisterSummaryView> {
        let views = if EnvOps::is_root() {
            // Root derives children from the registry (not the local cache).
            let snapshot = SubnetRegistryOps::snapshot();
            let children =
                ChildrenMapper::from_registry_snapshot(&snapshot, IcOps::canister_self());

            ChildrenMapper::snapshot_to_views(children)
        } else {
            // Non-root uses the cached children populated by topology cascade.
            let snapshot = CanisterChildrenOps::snapshot();

            ChildrenMapper::snapshot_to_views(snapshot)
        };

        // 3. Paginate in workflow
        paginate_vec(views, page)
    }
}
