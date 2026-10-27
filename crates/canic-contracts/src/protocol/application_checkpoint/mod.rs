//! Root checkpoint coordination and application-owned participant hook contracts.

/// Root's original whole-unit reservation.
pub const BEGIN: u64 = 0x4341_4e49_0000_0700;
/// Read retained Root progress without dispatch.
pub const STATUS: u64 = 0x4341_4e49_0000_0701;
/// Advance at most one original participant.
pub const ADVANCE: u64 = 0x4341_4e49_0000_0702;
/// Retain release intent, including unanswered prepares.
pub const RELEASE: u64 = 0x4341_4e49_0000_0703;
/// Application-owned idempotent prepare/release hook.
pub const PARTICIPANT: u64 = 0x4341_4e49_0000_0704;
/// Exact application endpoint called only by its installed Root.
pub const PARTICIPANT_METHOD: &str = "canic_application_checkpoint";

/// Fixed identities and compact progress; participant catalogues remain at Root.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 16_384,
    nodes: 1024,
    depth: 16,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_and_observation_frames_have_distinct_identities() {
        use crate::protocol::{
            cycle_balance, cycle_history, cycle_topups, protected_health, protected_logs,
            root_observation,
        };
        let checkpoint = [BEGIN, STATUS, ADVANCE, RELEASE, PARTICIPANT];
        let observation = [
            cycle_balance::METHOD_ID,
            cycle_history::METHOD_ID,
            cycle_topups::METHOD_ID,
            protected_health::METHOD_ID,
            protected_logs::METHOD_ID,
            root_observation::child_funding::METHOD_ID,
            root_observation::cycle_balance::METHOD_ID,
            root_observation::cycle_history::METHOD_ID,
            root_observation::cycle_topups::METHOD_ID,
            root_observation::memory_allocations::METHOD_ID,
            root_observation::metrics::METHOD_ID,
        ];
        let mut ids = std::collections::BTreeSet::new();
        for id in checkpoint.into_iter().chain(observation) {
            assert!(ids.insert(id));
        }
    }
}
