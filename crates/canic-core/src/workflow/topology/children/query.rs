use crate::{
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    ops::{
        runtime::env,
        storage::{children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
        prelude::*, topology::children::mapper::ChildrenMapper, view::paginate::paginate_vec,
    },
};

pub fn canister_children_page(page: PageRequest) -> Page<CanisterSummaryView> {
    let views = if env::is_root() {
        // Root derives children from the registry (not the local cache).
        let snapshot = SubnetRegistryOps::snapshot();
        let children = ChildrenMapper::from_registry_snapshot(&snapshot, canister_self());

        ChildrenMapper::snapshot_to_views(children)
    } else {
        // Non-root uses the cached children populated by topology cascade.
        let snapshot = CanisterChildrenOps::snapshot();

        ChildrenMapper::snapshot_to_views(snapshot)
    };

    // 3. Paginate in workflow
    paginate_vec(views, page)
}
