use crate::{
    dto::{
        log::LogEntryView,
        page::{Page, PageRequest},
    },
    log::Level,
    ops::runtime::log::LogViewOps,
};

pub(crate) fn log_page(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Page<LogEntryView> {
    LogViewOps::page(crate_name, topic, min_level, page)
}
