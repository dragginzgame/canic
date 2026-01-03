use crate::{
    dto::{
        cycles::CycleTrackerEntryView,
        page::{Page, PageRequest},
    },
    ops::storage::cycles::CycleTrackerOps,
    workflow::view::paginate::paginate_vec,
};

pub fn cycle_tracker_page(page: PageRequest) -> Page<CycleTrackerEntryView> {
    let snapshot = CycleTrackerOps::snapshot();
    let page = paginate_vec(snapshot.entries, page);
    let entries = page
        .entries
        .into_iter()
        .map(|(timestamp_secs, cycles)| CycleTrackerEntryView {
            timestamp_secs,
            cycles,
        })
        .collect();

    Page {
        entries,
        total: page.total,
    }
}
