use crate::{
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    ops::storage::children::CanisterChildrenOps,
    workflow::query::paginate::paginate_vec,
};

pub(crate) fn canister_children_page(page: PageRequest) -> Page<CanisterSummaryView> {
    // 1. Snapshot (stable, ordered)
    let snapshot = CanisterChildrenOps::snapshot();

    // 2. Project snapshot entries → views
    let views: Vec<CanisterSummaryView> = snapshot
        .entries
        .into_iter()
        .map(|(_pid, summary)| CanisterSummaryView::from(summary))
        .collect();

    // 3. Paginate in workflow
    paginate_vec(views, page)
}
