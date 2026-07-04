//! Module: ops::runtime::bootstrap
//!
//! Responsibility: track runtime bootstrap readiness diagnostics.
//! Does not own: lifecycle orchestration, readiness policy, or status DTO schema.
//! Boundary: stores process-local bootstrap status for query projection.

use super::recent_failure::{RecentFailureInput, RecentFailureOps};
use crate::{
    cdk::utils::time::now_nanos,
    dto::{runtime::FailureSeverity, state::BootstrapStatusResponse},
};
use std::cell::RefCell;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BootstrapStatusRecord {
    ready: bool,
    phase: &'static str,
    last_error: Option<String>,
}

thread_local! {
    static BOOTSTRAP_STATUS: RefCell<BootstrapStatusRecord> = const { RefCell::new(BootstrapStatusRecord {
        ready: false,
        phase: "idle",
        last_error: None,
    }) };
}

///
/// BootstrapStatusOps
///
/// Operations-layer facade for process-local bootstrap status diagnostics.
///

pub struct BootstrapStatusOps;

impl BootstrapStatusOps {
    // Return the current runtime bootstrap diagnostic snapshot.
    #[must_use]
    pub fn snapshot() -> BootstrapStatusResponse {
        BOOTSTRAP_STATUS.with_borrow(|status| BootstrapStatusResponse {
            ready: status.ready,
            phase: status.phase.to_string(),
            last_error: status.last_error.clone(),
        })
    }

    // Reset bootstrap progress to one new phase and clear any previous error.
    pub fn set_phase(phase: &'static str) {
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            status.ready = false;
            status.phase = phase;
            status.last_error = None;
        });
    }

    // Record one terminal bootstrap failure for diagnostics.
    pub fn mark_failed(message: impl Into<String>) {
        let message = message.into();
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            let failed_phase = status.phase;
            status.ready = false;
            status.phase = "failed";
            status.last_error = Some(message);
            RecentFailureOps::record(RecentFailureInput {
                occurred_at_ns: now_nanos(),
                subsystem: "runtime_bootstrap".to_string(),
                code: "bootstrap_failed".to_string(),
                severity: FailureSeverity::Error,
                summary: "bootstrap failed; inspect canic_bootstrap_status for details".to_string(),
                correlation_id: Some(failed_phase.to_string()),
            });
        });
    }

    // Record successful bootstrap completion.
    pub fn mark_ready() {
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            status.ready = true;
            status.phase = "ready";
            status.last_error = None;
        });
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::BootstrapStatusOps;
    use crate::ops::runtime::recent_failure::RecentFailureOps;

    #[test]
    fn bootstrap_status_starts_idle_and_not_ready() {
        BootstrapStatusOps::set_phase("idle");

        let status = BootstrapStatusOps::snapshot();

        assert!(!status.ready);
        assert_eq!(status.phase, "idle");
        assert_eq!(status.last_error, None);
    }

    #[test]
    fn bootstrap_status_tracks_failure() {
        RecentFailureOps::reset();
        BootstrapStatusOps::mark_failed("nope");

        let status = BootstrapStatusOps::snapshot();

        assert!(!status.ready);
        assert_eq!(status.phase, "failed");
        assert_eq!(status.last_error.as_deref(), Some("nope"));

        let failures = RecentFailureOps::snapshot();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].subsystem, "runtime_bootstrap");
        assert_eq!(failures[0].code, "bootstrap_failed");
        assert_eq!(
            failures[0].severity,
            crate::dto::runtime::FailureSeverity::Error
        );
        assert_eq!(
            failures[0].summary,
            "bootstrap failed; inspect canic_bootstrap_status for details"
        );
        assert!(!failures[0].summary.contains("nope"));

        RecentFailureOps::reset();
    }

    #[test]
    fn bootstrap_status_tracks_ready() {
        BootstrapStatusOps::set_phase("root:init:validate");
        BootstrapStatusOps::mark_ready();

        let status = BootstrapStatusOps::snapshot();

        assert!(status.ready);
        assert_eq!(status.phase, "ready");
        assert_eq!(status.last_error, None);
    }
}
