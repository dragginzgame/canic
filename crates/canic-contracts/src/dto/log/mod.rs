//
// LogEntry
//

use crate::{dto::prelude::*, log::Level};

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct LogEntry {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}
