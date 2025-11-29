use crate::{
    log::Level,
    model::memory::log::{LogEntryView, StableLog},
};
use candid::CandidType;
use serde::Serialize;

///
/// LogPageDto
///

#[derive(CandidType, Serialize)]
pub struct LogPageDto {
    pub entries: Vec<LogEntryView>,
    pub total: u64,
}

///
/// LogOps
///

pub struct LogOps;

impl LogOps {
    ///
    /// Export a page of log entries and the total count.
    ///
    #[must_use]
    pub fn page(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        offset: u64,
        limit: u64,
    ) -> LogPageDto {
        let (entries, total) = StableLog::entries_page_filtered(
            crate_name.as_deref(),
            topic.as_deref(),
            min_level,
            offset,
            limit,
        );

        LogPageDto { entries, total }
    }
}
