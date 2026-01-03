use crate::{
    PublicError,
    dto::{
        log::LogEntryView,
        page::{Page, PageRequest},
    },
    log::Level,
    workflow,
};

pub fn canic_log(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Result<Page<LogEntryView>, PublicError> {
    Ok(workflow::log::query::log_page(
        crate_name, topic, min_level, page,
    ))
}
