//! Module: workflow::runtime::log
//!
//! Responsibility: schedule and run runtime log retention sweeps.
//! Does not own: log storage schemas, retention policy rules, or endpoint authorization.
//! Boundary: runtime workflow timer coordinating log policy and log ops.

use crate::{
    domain::policy::pure as policy,
    log,
    log::Topic,
    ops::{config::ConfigOps, ic::IcOps, runtime::log::LogOps},
    workflow::{
        config::{WORKFLOW_INIT_DELAY, WORKFLOW_LOG_RETENTION_INTERVAL},
        runtime::timer::{TimerKey, TimerWorkflow},
    },
};
use std::time::Duration;

const RETENTION_INTERVAL: Duration = WORKFLOW_LOG_RETENTION_INTERVAL;

///
/// LogRetentionWorkflow
///

pub struct LogRetentionWorkflow;

impl LogRetentionWorkflow {
    /// Start periodic log retention sweeps.
    pub fn start() {
        TimerWorkflow::ensure_recurring(
            TimerKey::LogRetention,
            WORKFLOW_INIT_DELAY,
            RETENTION_INTERVAL,
            || async {
                let _ = Self::retain();
            },
        );
    }

    /// Run a retention sweep immediately.
    #[must_use]
    pub fn retain() -> bool {
        let cfg = match ConfigOps::log_config() {
            Ok(cfg) => cfg,
            Err(err) => {
                log!(Topic::Memory, Warn, "log retention skipped: {err}");
                return false;
            }
        };
        let now = IcOps::now_secs();
        let params = policy::log::retention_params(
            policy::log::LogRetentionPolicyInput {
                max_entries: cfg.max_entries,
                max_entry_bytes: cfg.max_entry_bytes,
                max_age_secs: cfg.max_age_secs,
            },
            now,
        );

        match LogOps::apply_retention(params.cutoff, params.max_entries, params.max_entry_bytes) {
            Ok(summary) => {
                let dropped = summary.dropped_total();
                if dropped > 0 {
                    log!(
                        Topic::Memory,
                        Info,
                        "log retention: dropped={}, before={}, retained={}",
                        dropped,
                        summary.before,
                        summary.retained
                    );
                }
                true
            }
            Err(err) => {
                log!(Topic::Memory, Warn, "log retention failed: {err}");
                false
            }
        }
    }
}
