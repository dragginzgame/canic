use crate::{dto::prelude::*, log::Level};

///
/// LogEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}
