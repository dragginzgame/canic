//! Short progress reporting for the Fleet Registry PocketIC journey.

use std::io::Write;

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub(super) fn progress(phase: &str) {
    eprintln!("[pic_fleet_registry] fixture: {phase}");
    let _ = std::io::stderr().flush();
}
