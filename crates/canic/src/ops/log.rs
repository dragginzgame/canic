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
    pub fn page(offset: u64, limit: u64, min_level: Level) -> LogPageDto {
        let entries = StableLog::entries_page_level(offset, limit, min_level);
        let total = StableLog::len();

        LogPageDto { entries, total }
    }
}
