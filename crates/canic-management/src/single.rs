//! Original endpoint contracts retain their exact first-slot meaning.
use crate::engine;
use canic_management_protocol::{Approval, Refusal, Stage, Status, children::Slot, upgrade};

/// Read the original child's evidence.
/// # Panics
/// Traps before lifecycle initialization.
#[must_use]
pub fn status() -> Status {
    engine::status(Slot::First)
}
/// Admit the original child.
/// # Errors
/// Refuses invalid authority, occupied custody, artifact, time or reserve.
/// # Panics
/// Traps for noncontrollers or unavailable storage.
pub fn stage(input: Stage) -> Result<Status, Refusal> {
    engine::stage(Slot::First, input)
}
/// Reconcile the original child's exact approval.
/// # Panics
/// Traps for noncontrollers, mismatched authority or unavailable storage.
pub async fn reconcile(expected: Approval) -> Status {
    engine::reconcile(Slot::First, expected).await
}
/// Read the original child's latest upgrade.
/// # Panics
/// Traps before lifecycle initialization.
#[must_use]
pub fn upgrade_status() -> upgrade::Status {
    engine::upgrade::status(Slot::First)
}
/// Read the original child's exact retained upgrade.
/// # Panics
/// Traps before lifecycle initialization.
#[must_use]
pub fn observe_upgrade(expected: &upgrade::Approval) -> upgrade::Status {
    engine::upgrade::observe(Slot::First, expected)
}
/// Admit an upgrade of the original child.
/// # Errors
/// Refuses invalid authority, occupied custody, artifact, time or reserve.
/// # Panics
/// Traps for noncontrollers or unavailable storage.
pub fn stage_upgrade(input: upgrade::Stage) -> Result<upgrade::Status, Refusal> {
    engine::upgrade::stage(Slot::First, input)
}
/// Reconcile the original child's exact upgrade approval.
/// # Panics
/// Traps for noncontrollers, mismatched authority or unavailable storage.
pub async fn reconcile_upgrade(expected: upgrade::Approval) -> upgrade::Status {
    engine::upgrade::reconcile(Slot::First, expected).await
}
/// Interrupt only the original child's test-only effect boundary.
/// # Panics
/// Traps for noncontrollers or an unsupported fault point.
#[cfg(feature = "fault-injection")]
pub fn inject_fault(value: u8) {
    engine::inject_fault(Slot::First, value);
}
