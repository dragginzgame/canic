use crate::{
    dto::{
        log::LogEntry,
        page::{Page, PageRequest},
    },
    log::Level,
    ops::runtime::log::LogOps,
    workflow::view::paginate::paginate_vec,
};

///
/// LogQuery
/// Read-only log views and pagination helpers.
///

pub struct LogQuery;

impl LogQuery {
    #[must_use]
    pub fn page(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        page: PageRequest,
    ) -> Page<LogEntry> {
        let mut entries =
            LogOps::snapshot_filtered(crate_name.as_deref(), topic.as_deref(), min_level);

        // Newest first
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        paginate_vec(entries, page)
    }
}
