use crate::{
    dto::{
        log::LogEntryView,
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
    ) -> Page<LogEntryView> {
        let mut entries = LogOps::snapshot();

        // Filter
        if let Some(ref name) = crate_name {
            entries.retain(|e| &e.crate_name == name);
        }
        if let Some(ref t) = topic {
            entries.retain(|e| e.topic.as_deref() == Some(t.as_str()));
        }
        if let Some(min) = min_level {
            entries.retain(|e| e.level >= min);
        }

        // Newest first
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        paginate_vec(entries, page)
    }
}
