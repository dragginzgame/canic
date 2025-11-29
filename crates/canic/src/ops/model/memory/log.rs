use crate::{
    log::Level,
    model::memory::log::{LogEntry, StableLog},
};
use candid::CandidType;
use serde::Serialize;

///
/// LogEntryDto
///

#[derive(CandidType, Clone, Debug, Serialize)]
pub struct LogEntryDto {
    pub index: u64,
    pub created_at: u64,
    pub crate_name: String,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntryDto {
    fn from_pair(index: usize, entry: LogEntry) -> Self {
        Self {
            index: index as u64,
            created_at: entry.created_at,
            crate_name: entry.crate_name,
            level: entry.level,
            topic: entry.topic,
            message: entry.message,
        }
    }
}

///
/// LogPageDto
///

#[derive(CandidType, Serialize)]
pub struct LogPageDto {
    pub entries: Vec<LogEntryDto>,
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
