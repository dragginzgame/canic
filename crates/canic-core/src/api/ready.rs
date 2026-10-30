//! Module: api::ready
//!
//! Responsibility: public readiness facade for endpoints and deferred application work.
//! Does not own: bootstrap state mutation or readiness barrier internals.
//! Boundary: exposes read-only readiness, current work admission and bootstrap status.

use crate::{
    dto::state::BootstrapStatusResponse,
    ops::runtime::{bootstrap::BootstrapStatusOps, ready::ReadyOps},
};

///
/// ReadyApi
///
/// Thin endpoint-facing facade for readiness checks.
///

pub struct ReadyApi;

impl ReadyApi {
    /// Mark a synchronous built-in lifecycle adapter ready after all invariants commit.
    pub fn mark_ready() {
        ReadyOps::mark_ready();
    }

    /// Return whether Canic runtime invariants have completed restoration.
    #[must_use]
    pub fn is_ready() -> bool {
        ReadyOps::is_ready()
    }

    /// Return current eligibility for a new deferred application effect.
    ///
    /// Requires restored runtime invariants, an unsealed timer authority, active
    /// managed admission and completed application startup where required by the
    /// role. Standalone-local mode keeps its existing activation/startup exemption.
    /// This read does not initialize workers, authorize a caller or establish peer
    /// freshness. Recheck immediately before effects and after each await. A false
    /// result does not cancel an existing call or prevent its original settlement.
    ///
    /// # Panics
    /// Storage and invariant traps from the authoritative owners propagate.
    #[must_use]
    pub fn is_application_work_admitted() -> bool {
        crate::workflow::runtime::application_startup::is_application_work_admitted()
    }

    /// Return the current bootstrap readiness snapshot.
    #[must_use]
    pub fn bootstrap_status() -> BootstrapStatusResponse {
        BootstrapStatusOps::snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::ReadyApi;
    use crate::{
        ops::runtime::fleet_activation::FleetActivationRuntimeOps,
        workflow::runtime::timer::TimerAuthorityWorkflow,
    };

    #[test]
    fn standalone_application_work_requires_restore_and_an_open_snapshot_fence() {
        FleetActivationRuntimeOps::set_standalone_local();
        assert!(!ReadyApi::is_application_work_admitted());
        ReadyApi::mark_ready();
        assert!(ReadyApi::is_ready());
        assert!(ReadyApi::is_application_work_admitted());

        TimerAuthorityWorkflow::restore_snapshot_suspension(true);
        assert!(ReadyApi::is_ready());
        assert!(!ReadyApi::is_application_work_admitted());

        TimerAuthorityWorkflow::restore_snapshot_suspension(false);
        assert!(ReadyApi::is_application_work_admitted());
    }

    #[test]
    fn restored_managed_runtime_without_activation_cannot_run_application_work() {
        ReadyApi::mark_ready();
        assert!(ReadyApi::is_ready());
        assert!(!ReadyApi::is_application_work_admitted());
    }
}
