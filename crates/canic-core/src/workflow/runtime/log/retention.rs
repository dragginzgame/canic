use crate::{
    log,
    log::Topic,
    ops::{
        OPS_INIT_DELAY, OPS_LOG_RETENTION_INTERVAL,
        runtime::{
            log::apply_log_retention,
            timer::{TimerId, TimerOps},
        },
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static RETENTION_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const RETENTION_INTERVAL: Duration = OPS_LOG_RETENTION_INTERVAL;

/// Start periodic log retention sweeps.
pub fn start() {
    let _ = TimerOps::set_guarded_interval(
        &RETENTION_TIMER,
        OPS_INIT_DELAY,
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
pub fn stop() {
    let _ = TimerOps::clear_guarded(&RETENTION_TIMER);
}

/// Run a retention sweep immediately.
#[must_use]
pub fn retain() -> bool {
    match apply_log_retention() {
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
