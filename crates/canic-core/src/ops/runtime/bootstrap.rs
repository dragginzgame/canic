use crate::dto::state::BootstrapStatusResponse;
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
        BOOTSTRAP_STATUS.with_borrow_mut(|status| {
            status.ready = false;
            status.phase = "failed";
            status.last_error = Some(message.into());
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

#[cfg(test)]
mod tests {
    use super::BootstrapStatusOps;

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
        BootstrapStatusOps::mark_failed("nope");

        let status = BootstrapStatusOps::snapshot();

        assert!(!status.ready);
        assert_eq!(status.phase, "failed");
        assert_eq!(status.last_error.as_deref(), Some("nope"));
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
