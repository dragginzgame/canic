//! Short progress reporting for the Fleet Registry PocketIC journey.

use std::time::Instant;

use crate::pic::progress::{self, ProgressStatus};

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub(super) fn progress(phase: &str) {
    progress::event("FLEET", ProgressStatus::Run, phase);
}

// Emit one completed Fleet-fixture phase without a measured duration.
pub(super) fn progress_ready(phase: &str) {
    progress::event("FLEET", ProgressStatus::Ready, phase);
}

// Emit one completed Fleet-fixture phase with its wall-clock duration.
pub(super) fn progress_elapsed(phase: &str, started_at: Instant) {
    progress::timed("FLEET", ProgressStatus::Done, phase, started_at.elapsed());
}
