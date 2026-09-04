//! Module: ops::runtime::bootstrap
//!
//! Responsibility: track runtime bootstrap readiness diagnostics.
//! Does not own: lifecycle orchestration, readiness policy, or status DTO schema.
//! Boundary: stores process-local bootstrap status for query projection.

use super::recent_failure::{RecentFailureInput, RecentFailureOps};
use crate::{
    domain::runtime::FailureSeverity, dto::state::BootstrapStatusResponse, ops::ic::IcOps,
};
use std::cell::RefCell;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BootstrapStatusRecord {
    ready: bool,
    phase: BootstrapPhaseLabel,
    last_error: Option<String>,
    retryable_init_failure_exhausted: bool,
}

/// Runtime-owned bootstrap diagnostic phase label.
/// Serialized bootstrap status responses still expose the label as a string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapPhaseLabel(&'static str);

impl BootstrapPhaseLabel {
    pub const IDLE: Self = Self("idle");
    pub const FAILED: Self = Self("failed");
    pub const READY: Self = Self("ready");
    pub const NONROOT_INIT_SCHEDULED: Self = Self("nonroot:init:scheduled");
    pub const NONROOT_INIT: Self = Self("nonroot:init");
    pub const NONROOT_INIT_WAITING_AUTHORITY: Self = Self("nonroot:init:waiting_authority");
    pub const NONROOT_UPGRADE_SCHEDULED: Self = Self("nonroot:upgrade:scheduled");
    pub const NONROOT_UPGRADE: Self = Self("nonroot:upgrade");
    pub const NONROOT_UPGRADE_WAITING_AUTHORITY: Self = Self("nonroot:upgrade:waiting_authority");
    pub const ROOT_INIT: Self = Self("root:init");
    pub const ROOT_INIT_WAITING_STAGED_RELEASES: Self = Self("root:init:waiting_staged_releases");
    pub const ROOT_INIT_WAITING_COMPONENT_REGISTRY: Self =
        Self("root:init:waiting_component_registry");
    pub const ROOT_INIT_SKIPPED: Self = Self("root:init:skipped");
    pub const ROOT_INIT_SET_SUBNET_ID: Self = Self("root:init:set_subnet_id");
    pub const ROOT_INIT_ACTIVATION_PREPARED: Self = Self("root:init:activation_prepared");
    pub const ROOT_UPGRADE_WAITING_STAGED_RELEASES: Self =
        Self("root:upgrade:waiting_staged_releases");
    pub const ROOT_UPGRADE_SET_SUBNET_ID: Self = Self("root:upgrade:set_subnet_id");
    pub const ROOT_UPGRADE_RECONCILE_WASM_STORE: Self = Self("root:upgrade:reconcile_wasm_store");

    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self(label)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

thread_local! {
    static BOOTSTRAP_STATUS: RefCell<BootstrapStatusRecord> = const { RefCell::new(BootstrapStatusRecord {
        ready: false,
        phase: BootstrapPhaseLabel::IDLE,
        last_error: None,
        retryable_init_failure_exhausted: false,
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
            phase: status.phase.as_str().to_string(),
            last_error: status.last_error.clone(),
        })
    }

    // Reset bootstrap progress to one new phase and clear any previous error.
    pub fn set_phase(phase: BootstrapPhaseLabel) {
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            status.ready = false;
            status.phase = phase;
            status.last_error = None;
            status.retryable_init_failure_exhausted = false;
        });
    }

    /// Claim the sole process-local initial-bootstrap scheduler.
    ///
    /// A later runtime-configuration command may call this after an earlier
    /// transient failure; concurrent callbacks remain collapsed to one owner.
    #[must_use]
    pub fn try_schedule_nonroot_init() -> bool {
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            let initial_schedule = status.phase == BootstrapPhaseLabel::IDLE;
            let retry_exhausted = status.phase == BootstrapPhaseLabel::FAILED
                && status.retryable_init_failure_exhausted;
            if status.ready || !(initial_schedule || retry_exhausted) {
                return false;
            }
            status.phase = BootstrapPhaseLabel::NONROOT_INIT_SCHEDULED;
            status.last_error = None;
            status.retryable_init_failure_exhausted = false;
            true
        })
    }

    // Record one terminal bootstrap failure for diagnostics.
    pub fn mark_failed(message: impl Into<String>) {
        Self::record_failure(message.into(), false);
    }

    /// Retain one exhausted transient init failure for an exact later
    /// runtime-configuration replay to reclaim.
    pub fn mark_retryable_init_failure_exhausted(message: impl Into<String>) {
        Self::record_failure(message.into(), true);
    }

    fn record_failure(message: String, retryable_init_failure_exhausted: bool) {
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            let failed_phase = status.phase;
            status.ready = false;
            status.phase = BootstrapPhaseLabel::FAILED;
            status.last_error = Some(message);
            status.retryable_init_failure_exhausted = retryable_init_failure_exhausted;
            RecentFailureOps::record(RecentFailureInput {
                occurred_at_ns: IcOps::now_nanos(),
                subsystem: "runtime_bootstrap".to_string(),
                code: "bootstrap_failed".to_string(),
                severity: FailureSeverity::Error,
                summary: "bootstrap failed; inspect the role-owned Overview status for details"
                    .to_string(),
                correlation_id: Some(failed_phase.as_str().to_string()),
            });
        });
    }

    // Record successful bootstrap completion.
    pub fn mark_ready() {
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            status.ready = true;
            status.phase = BootstrapPhaseLabel::READY;
            status.last_error = None;
            status.retryable_init_failure_exhausted = false;
        });
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{BootstrapPhaseLabel, BootstrapStatusOps};
    use crate::ops::runtime::recent_failure::RecentFailureOps;

    #[test]
    fn bootstrap_status_starts_idle_and_not_ready() {
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::IDLE);

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
            crate::domain::runtime::FailureSeverity::Error
        );
        assert_eq!(
            failures[0].summary,
            "bootstrap failed; inspect the role-owned Overview status for details"
        );
        assert!(!failures[0].summary.contains("nope"));

        RecentFailureOps::reset();
    }

    #[test]
    fn bootstrap_status_tracks_ready() {
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::new("root:init:validate"));
        BootstrapStatusOps::mark_ready();

        let status = BootstrapStatusOps::snapshot();

        assert!(status.ready);
        assert_eq!(status.phase, "ready");
        assert_eq!(status.last_error, None);
    }

    #[test]
    fn exhausted_transient_init_failure_is_reclaimed_exactly_once() {
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT);
        BootstrapStatusOps::mark_retryable_init_failure_exhausted("authority unavailable");

        assert!(BootstrapStatusOps::try_schedule_nonroot_init());
        assert!(!BootstrapStatusOps::try_schedule_nonroot_init());

        let status = BootstrapStatusOps::snapshot();
        assert!(!status.ready);
        assert_eq!(status.phase, "nonroot:init:scheduled");
        assert_eq!(status.last_error, None);
    }

    #[test]
    fn reclaimed_transient_init_can_reach_ready_and_terminal_replay_is_effect_free() {
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT);
        BootstrapStatusOps::mark_retryable_init_failure_exhausted("Root unavailable");
        assert!(BootstrapStatusOps::try_schedule_nonroot_init());

        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT);
        BootstrapStatusOps::mark_ready();

        let status = BootstrapStatusOps::snapshot();
        assert!(status.ready);
        assert_eq!(status.phase, "ready");
        assert_eq!(status.last_error, None);
        assert!(!BootstrapStatusOps::try_schedule_nonroot_init());
    }

    #[test]
    fn nonretryable_init_failure_remains_terminal_on_runtime_replay() {
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT);
        BootstrapStatusOps::mark_failed("invalid authority");

        assert!(!BootstrapStatusOps::try_schedule_nonroot_init());

        let status = BootstrapStatusOps::snapshot();
        assert!(!status.ready);
        assert_eq!(status.phase, "failed");
        assert_eq!(status.last_error.as_deref(), Some("invalid authority"));
    }

    #[test]
    fn post_upgrade_bootstrap_cannot_be_claimed_by_init_replay() {
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_UPGRADE_WAITING_AUTHORITY);

        assert!(!BootstrapStatusOps::try_schedule_nonroot_init());

        let status = BootstrapStatusOps::snapshot();
        assert!(!status.ready);
        assert_eq!(status.phase, "nonroot:upgrade:waiting_authority");
        assert_eq!(status.last_error, None);
    }
}
