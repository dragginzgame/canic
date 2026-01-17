use crate::{
    ops::{ic::IcOps, storage::intent::IntentStoreOps},
    workflow::{
        config::{WORKFLOW_INIT_DELAY, WORKFLOW_INTENT_CLEANUP_INTERVAL},
        prelude::*,
        runtime::timer::{TimerId, TimerWorkflow},
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static INTENT_CLEANUP_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const CLEANUP_INTERVAL: Duration = WORKFLOW_INTENT_CLEANUP_INTERVAL;

///
/// IntentCleanupWorkflow
///

pub struct IntentCleanupWorkflow;

impl IntentCleanupWorkflow {
    /// Start periodic intent cleanup sweeps.
    pub fn start() {
        let _ = TimerWorkflow::set_guarded_interval(
            &INTENT_CLEANUP_TIMER,
            WORKFLOW_INIT_DELAY,
            "intent_cleanup:init",
            || async {
                let _ = Self::cleanup();
            },
            CLEANUP_INTERVAL,
            "intent_cleanup:interval",
            || async {
                let _ = Self::cleanup();
            },
        );
    }

    /// Stop periodic cleanup sweeps.
    #[expect(dead_code)]
    pub fn stop() {
        let _ = TimerWorkflow::clear_guarded(&INTENT_CLEANUP_TIMER);
    }

    /// Run a cleanup sweep immediately.
    #[must_use]
    pub fn cleanup() -> bool {
        let now = IcOps::now_secs();
        let expired = IntentStoreOps::list_expired_pending_intents(now);

        if expired.is_empty() {
            return true;
        }

        let expired_total = expired.len();
        let mut aborted = 0usize;
        let mut errors = 0usize;

        for intent_id in expired {
            match IntentStoreOps::abort_intent_if_pending(intent_id) {
                Ok(true) => aborted += 1,
                Ok(false) => {}
                Err(err) => {
                    errors += 1;
                    log!(
                        Topic::Memory,
                        Warn,
                        "intent cleanup abort failed id={intent_id}: {err}"
                    );
                }
            }
        }

        log!(
            Topic::Memory,
            Info,
            "intent cleanup: expired={expired_total} aborted={aborted} errors={errors}"
        );

        errors == 0
    }
}
