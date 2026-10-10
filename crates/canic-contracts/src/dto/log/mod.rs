//
// LogEntry
//

use crate::{dto::prelude::*, log::Level};

#[derive(CandidType, Deserialize)]
pub struct LogEntry {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}
