//! Durable business-attempt accounting embedded in each operation's existing owner.

use serde::{Deserialize, Serialize};

/// Current retry window; exact replay and restart preserve every charged attempt.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContinuationRetryRecord {
    pub generation: u64,
    pub attempts_without_progress: u32,
    pub retry_at_ns: u64,
    pub diagnostic_code: Option<u16>,
    pub review_required: bool,
}

impl ContinuationRetryRecord {
    /// Reserve the maximum diagnostic representation when the operation is admitted.
    #[cfg(feature = "root-control-plane")]
    pub const MAXIMUM: Self = Self {
        generation: u64::MAX,
        attempts_without_progress: u32::MAX,
        retry_at_ns: u64::MAX,
        diagnostic_code: Some(u16::MAX),
        review_required: true,
    };
}
