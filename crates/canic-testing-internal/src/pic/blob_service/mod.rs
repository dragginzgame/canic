//! Framework-owned dedicated and embedded Blob composition qualification.

pub(super) mod artifacts;
mod journey;
mod metrics;
mod tests;

pub(super) fn governed_cases() -> Vec<crate::pic::cases::GovernedTestCase> {
    crate::pic::cases::registered![
        (
            "dedicated Blob composition preserves permissions and recovery",
            tests::managed_composition_dedicated
        ),
        (
            "embedded Blob composition preserves application state and recovery",
            tests::managed_composition_embedded
        ),
    ]
}
