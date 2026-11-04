//
// LogEntry
//

use crate::{dto::prelude::*, log::Level};

#[derive(Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LogEntry {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

/// One bounded newest-first scan; the cursor is exclusive and remains valid across eviction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LogPageRequest {
    pub before: Option<u64>,
    pub limit: u16,
}

/// Bounded diagnostics; empty filtered scans can still require continuation.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LogPage {
    pub entries: Vec<LogEntry>,
    /// Candidate rows examined, rather than an exact full-history match count.
    pub scanned: u32,
    pub next_before: Option<u64>,
}
