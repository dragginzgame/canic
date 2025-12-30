use crate::{dto::log::LogEntryView, model::memory::log::LogEntry};

#[must_use]
pub fn log_entry_to_view(entry: &LogEntry) -> LogEntryView {
    LogEntryView {
        crate_name: entry.crate_name.clone(),
        created_at: entry.created_at,
        level: entry.level,
        topic: entry.topic.clone(),
        message: entry.message.clone(),
    }
}
