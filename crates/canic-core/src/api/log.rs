use crate::{
    dto::{
        log::LogEntryView,
        page::{Page, PageRequest},
    },
    log::Level,
    workflow::log::query::LogQuery,
};

///
/// LogApi
///

pub struct LogApi;

impl LogApi {
    #[must_use]
    pub fn entries(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        page: PageRequest,
    ) -> Page<LogEntryView> {
        LogQuery::page(crate_name, topic, min_level, page)
    }
}
