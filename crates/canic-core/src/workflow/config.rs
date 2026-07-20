//! Module: workflow::config
//!
//! Responsibility: provide shared workflow scheduling constants and config export.
//! Does not own: configuration storage, timer registration, or endpoint authorization.
//! Boundary: delegates config serialization to ops and centralizes workflow cadences.

use crate::{InternalError, ops::config::ConfigOps};
use std::time::Duration;

///
/// Workflow scheduling constants.
///

/// Shared initial delay for background workflow timers to allow init work to settle.
pub const WORKFLOW_INIT_DELAY: Duration = Duration::from_secs(30);

/// Shared cadence for cycle tracking (60 minutes).
pub const WORKFLOW_CYCLE_TRACK_INTERVAL: Duration = Duration::from_hours(1);

/// Root delegated-proof renewal sweep cadence (1 minute).
pub const WORKFLOW_AUTH_RENEWAL_INTERVAL: Duration = Duration::from_mins(1);

///
/// ConfigWorkflow
///
/// Workflow facade for configuration export.
///

pub struct ConfigWorkflow;

impl ConfigWorkflow {
    pub fn export_toml() -> Result<String, InternalError> {
        ConfigOps::export_toml()
    }
}
