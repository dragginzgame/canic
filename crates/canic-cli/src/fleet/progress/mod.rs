//! Module: fleet::progress
//!
//! Responsibility: bound repeated informational wait output with a periodic heartbeat.
//! Does not own: polling, reconciliation, effects, errors or review decisions.
//! Boundary: only unchanged waiting events are suppressed; elapsed time is not progress.

#[cfg(test)]
mod tests;

use canic_host::fleet_ensure::dto::{FleetEnsureProgress, FleetEnsureProgressState};
use std::time::{Duration, Instant};

const HEARTBEAT: Duration = Duration::from_secs(30);

/// Invocation-local last emitted state and monotonic heartbeat deadline.
#[derive(Default)]
pub(super) struct ProgressOutput {
    last_wait: Option<(FleetEnsureProgress, Instant)>,
}

impl ProgressOutput {
    /// Emit changes immediately; retain the latest elapsed detail for each heartbeat.
    pub(super) fn should_emit(&mut self, progress: &FleetEnsureProgress, now: Instant) -> bool {
        let FleetEnsureProgressState::AwaitingProgress { .. } = progress.state else {
            self.last_wait = None;
            return true;
        };
        let mut identity = progress.clone();
        if let FleetEnsureProgressState::AwaitingProgress {
            elapsed_seconds, ..
        } = &mut identity.state
        {
            *elapsed_seconds = 0;
        }
        if self.last_wait.as_ref().is_some_and(|(previous, emitted)| {
            previous == &identity && now.saturating_duration_since(*emitted) < HEARTBEAT
        }) {
            return false;
        }
        self.last_wait = Some((identity, now));
        true
    }
}
