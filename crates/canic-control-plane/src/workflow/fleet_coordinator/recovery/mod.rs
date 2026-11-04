//! Module: workflow::fleet_coordinator::recovery
//!
//! Responsibility: retain Coordinator's sole native recovery watchdog.
//! Does not own: operation journals, retry policy, or callback execution.
//! Boundary: every takeover rechecks domain demand and native running authority.

use canic_core::{api::timer::TimerApi, control_plane_support::error::InternalError};
use ic_timers::{
    TimerCadence, TimerCompletion, WatchdogDecision, WatchdogReconcileState, WatchdogRegistration,
    WatchdogRunResult, reconcile_watchdog,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static RECOVERY: RefCell<Option<WatchdogRegistration>> = const { RefCell::new(None) };
}

fn completion() -> TimerCompletion {
    let recovered = u64::from(super::provisioning_timer::recover())
        + u64::from(super::transition_timer::recover());
    if recovered == 0 {
        TimerCompletion::no_work()
    } else {
        TimerCompletion::success(recovered)
    }
}

/// Coordinator owns one watchdog; Root's pool is not part of this canister graph.
pub(super) fn ensure() -> Result<(), InternalError> {
    let identity = TimerApi::recovery_watchdog_identity().map_err(InternalError::from)?;
    let cadence =
        TimerCadence::new(Duration::from_secs(30)).map_err(|_| InternalError::invariant())?;
    RECOVERY.with(|slot| {
        let mut registration = slot
            .try_borrow_mut()
            .map_err(|_| InternalError::unavailable())?;
        reconcile_watchdog(
            &mut registration,
            &identity,
            cadence,
            WatchdogReconcileState::Scheduled,
            |_| WatchdogRunResult::new(completion(), WatchdogDecision::Continue),
        )
        .map_err(super::transition_timer::registration_error)
    })
}

/// Preserve the empty-inventory snapshot fence without retaining idle declarations.
pub(super) fn release_idle() -> Result<(), InternalError> {
    if super::provisioning_timer::has_pending()? || super::transition_timer::has_pending()? {
        return Err(InternalError::unavailable());
    }
    let release = || -> Result<(), InternalError> {
        super::provisioning_timer::unregister_idle()?;
        super::transition_timer::unregister_idle()?;
        RECOVERY.with(|slot| {
            let mut slot = slot
                .try_borrow_mut()
                .map_err(|_| InternalError::unavailable())?;
            if let Some(registration) = slot.take() {
                match registration.unregister() {
                    Ok(()) | Err(ic_timers::TimerError::RegistrationExpired) => {}
                    Err(error) => return Err(super::transition_timer::registration_error(error)),
                }
            }
            Ok(())
        })
    };
    // Consuming native claims must either finish together or roll back this message.
    release().unwrap_or_else(|error| {
        canic_core::control_plane_support::ops::ic::IcOps::trap(format!(
            "Coordinator idle timer cleanup failed: {error}"
        ))
    });
    Ok(())
}
