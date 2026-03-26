use crate::{
    dto::{
        cycles::CycleTrackerEntry,
        page::{Page, PageRequest},
    },
    ops::storage::cycles::CycleTrackerOps,
    workflow::view::paginate::paginate_vec,
};

///
/// CycleTrackerQuery
///

pub struct CycleTrackerQuery;

impl CycleTrackerQuery {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<CycleTrackerEntry> {
        let page = paginate_vec(CycleTrackerOps::entries(), page);
        let entries = page
            .entries
            .into_iter()
            .map(|(timestamp_secs, cycles)| CycleTrackerEntry {
                timestamp_secs,
                cycles,
            })
            .collect();

        Page {
            entries,
            total: page.total,
        }
    }
}
