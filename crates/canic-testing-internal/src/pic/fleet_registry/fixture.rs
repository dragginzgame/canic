//! Short progress reporting for the Fleet Registry PocketIC journey.

use std::{io::Write, time::Instant};

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub(super) fn progress(phase: &str) {
    eprintln!("[pic_fleet_registry] fixture: {phase}");
    let _ = std::io::stderr().flush();
}

// Emit one completed Fleet-fixture phase with its wall-clock duration.
pub(super) fn progress_elapsed(phase: &str, started_at: Instant) {
    progress(&format!(
        "{phase} elapsed={:.3}s",
        started_at.elapsed().as_secs_f64()
    ));
}
