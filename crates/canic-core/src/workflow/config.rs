use std::time::Duration;

///
/// Workflow scheduling constants.
///

/// Shared initial delay for workflow timers to allow init work to settle.
pub const WORKFLOW_INIT_DELAY: Duration = Duration::from_secs(10);

/// Shared cadence for cycle tracking (10 minutes).
pub const WORKFLOW_CYCLE_TRACK_INTERVAL: Duration = Duration::from_secs(60 * 10);

/// Shared cadence for log retention (10 minutes).
pub const WORKFLOW_LOG_RETENTION_INTERVAL: Duration = Duration::from_secs(60 * 10);

/// Pool timer initial delay (30 seconds) before first check.
pub const WORKFLOW_POOL_INIT_DELAY: Duration = Duration::from_secs(30);

/// Pool check cadence (30 minutes).
pub const WORKFLOW_POOL_CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);
