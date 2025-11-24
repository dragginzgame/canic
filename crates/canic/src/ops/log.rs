use crate::{
    log::Level,
    memory::log::{LogEntryView, StableLog},
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
        offset: u64,
        limit: u64,
        topic: Option<String>,
        min_level: Option<Level>,
    ) -> LogPageDto {
        let (entries, total) =
            StableLog::entries_page_filtered(offset, limit, topic.as_deref(), min_level);

        LogPageDto { entries, total }
    }
}
