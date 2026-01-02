use crate::{dto::log::LogEntryView, ops::runtime::log::LogEntrySnapshot};

pub struct LogMapper;

impl LogMapper {
    #[must_use]
    pub fn entry_to_view(entry: &LogEntrySnapshot) -> LogEntryView {
        LogEntryView {
            crate_name: entry.crate_name.clone(),
            created_at: entry.created_at,
            level: entry.level,
            topic: entry.topic.clone(),
            message: entry.message.clone(),
        }
    }
}
