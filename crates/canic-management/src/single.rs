//! Original endpoint contracts retain their exact first-slot meaning.
use crate::engine;
use canic_management_protocol::{Status, children::ChildId, upgrade};

/// Read the original child's evidence.
/// # Panics
/// Traps before lifecycle initialization.
#[must_use]
pub fn status() -> Status {
    engine::status(ChildId::FIRST)
}

/// Read the original child's latest upgrade.
/// # Panics
/// Traps before lifecycle initialization.
#[must_use]
pub fn upgrade_status() -> upgrade::Status {
    engine::upgrade::status(ChildId::FIRST)
}

/// Read the original child's exact retained upgrade.
/// # Panics
/// Traps before lifecycle initialization.
#[must_use]
pub fn observe_upgrade(expected: &upgrade::Approval) -> upgrade::Status {
    engine::upgrade::observe(ChildId::FIRST, expected)
}

/// Interrupt only the original child's test-only effect boundary.
/// # Panics
/// Traps for noncontrollers or an unsupported fault point.
#[cfg(feature = "fault-injection")]
pub fn inject_fault(value: u8) {
    engine::inject_fault(ChildId::FIRST, value);
}
