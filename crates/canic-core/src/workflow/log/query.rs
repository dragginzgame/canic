use crate::{
    dto::{
        log::LogEntry,
        page::{Page, PageRequest},
    },
    log::Level,
    ops::runtime::log::LogOps,
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
        LogOps::page_filtered(crate_name.as_deref(), topic.as_deref(), min_level, page)
    }
}
