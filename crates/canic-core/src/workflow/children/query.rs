use crate::{
    Error,
    access::env,
    cdk::types::Principal,
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    ops::storage::children::CanisterChildrenOps,
    workflow::{children::mapper::ChildrenMapper, view::paginate::paginate_vec},
};

pub(crate) fn canister_children_page(page: PageRequest) -> Page<CanisterSummaryView> {
    // 1. Snapshot (stable, ordered)
    let snapshot = CanisterChildrenOps::snapshot();

    // 2. Project snapshot entries → views
    let views = ChildrenMapper::snapshot_to_views(snapshot);

    // 3. Paginate in workflow
    paginate_vec(views, page)
}

pub(crate) fn child_pid_by_role(role: CanisterRole) -> Result<Option<Principal>, Error> {
    env::deny_root()?;

    Ok(CanisterChildrenOps::find_first_by_role(&role).map(|child| child.pid))
}
