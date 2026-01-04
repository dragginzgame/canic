use crate::{
    cdk::utils::time::now_secs,
    domain::policy,
    ops::{
        config::ConfigOps,
        runtime::{
            log::LogOps,
            timer::{TimerId, TimerOps},
        },
    },
    workflow::{
        config::{WORKFLOW_INIT_DELAY, WORKFLOW_LOG_RETENTION_INTERVAL},
        prelude::*,
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static RETENTION_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const RETENTION_INTERVAL: Duration = WORKFLOW_LOG_RETENTION_INTERVAL;

/// Start periodic log retention sweeps.
pub fn start() {
    let _ = TimerOps::set_guarded_interval(
        &RETENTION_TIMER,
        WORKFLOW_INIT_DELAY,
        "log_retention:init",
        || async {
            let _ = retain();
        },
        RETENTION_INTERVAL,
        "log_retention:interval",
        || async {
            let _ = retain();
        },
    );
}

/// Stop periodic retention sweeps.
#[expect(dead_code)]
pub fn stop() {
    let _ = TimerOps::clear_guarded(&RETENTION_TIMER);
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
    let now = now_secs();
    let params = policy::log::retention_params(&cfg, now);

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
