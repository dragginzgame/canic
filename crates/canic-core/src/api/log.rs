use crate::{
    dto::{
        log::LogEntryView,
        page::{Page, PageRequest},
    },
    log::Level,
    workflow,
};

///
/// Log API
///

#[must_use]
pub fn log(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Page<LogEntryView> {
    workflow::log::query::log_page(crate_name, topic, min_level, page)
}
