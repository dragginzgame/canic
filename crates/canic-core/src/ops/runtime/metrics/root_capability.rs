use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ROOT_CAPABILITY_METRICS: RefCell<HashMap<RootCapabilityMetricStorageKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// RootCapabilityMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[remain::sorted]
pub enum RootCapabilityMetricKey {
    IssueRoleAttestation,
    Provision,
    RecycleCanister,
    RequestCycles,
    Upgrade,
}

impl RootCapabilityMetricKey {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::IssueRoleAttestation => "IssueRoleAttestation",
            Self::Provision => "Provision",
            Self::RecycleCanister => "RecycleCanister",
            Self::RequestCycles => "RequestCycles",
            Self::Upgrade => "Upgrade",
        }
    }
}

///
/// RootCapabilityMetricEventType
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[remain::sorted]
pub enum RootCapabilityMetricEventType {
    Authorization,
    Envelope,
    Execution,
    Proof,
    Replay,
}

impl RootCapabilityMetricEventType {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Authorization => "Authorization",
            Self::Envelope => "Envelope",
            Self::Execution => "Execution",
            Self::Proof => "Proof",
            Self::Replay => "Replay",
        }
    }
}

///
/// RootCapabilityMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[remain::sorted]
pub enum RootCapabilityMetricOutcome {
    Accepted,
    Denied,
    DuplicateConflict,
    DuplicateSame,
    Error,
    Expired,
    Rejected,
    Success,
    TtlExceeded,
}

impl RootCapabilityMetricOutcome {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Accepted => "Accepted",
            Self::Denied => "Denied",
            Self::DuplicateConflict => "DuplicateConflict",
            Self::DuplicateSame => "DuplicateSame",
            Self::Error => "Error",
            Self::Expired => "Expired",
            Self::Rejected => "Rejected",
            Self::Success => "Success",
            Self::TtlExceeded => "TtlExceeded",
        }
    }
}

///
/// RootCapabilityMetricProofMode
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[remain::sorted]
pub enum RootCapabilityMetricProofMode {
    DelegatedGrant,
    RoleAttestation,
    Structural,
    Unspecified,
}

impl RootCapabilityMetricProofMode {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::DelegatedGrant => "DelegatedGrant",
            Self::RoleAttestation => "RoleAttestation",
            Self::Structural => "Structural",
            Self::Unspecified => "Unspecified",
        }
    }
}

///
/// RootCapabilityMetricStorageKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct RootCapabilityMetricStorageKey {
    capability: RootCapabilityMetricKey,
    event_type: RootCapabilityMetricEventType,
    outcome: RootCapabilityMetricOutcome,
    proof_mode: RootCapabilityMetricProofMode,
}

///
/// RootCapabilityMetrics
///

pub struct RootCapabilityMetrics;

impl RootCapabilityMetrics {
    /// record
    ///
    /// Record a root capability metric point using the flattened metric key.
    fn record(
        capability: RootCapabilityMetricKey,
        event_type: RootCapabilityMetricEventType,
        outcome: RootCapabilityMetricOutcome,
        proof_mode: RootCapabilityMetricProofMode,
    ) {
        ROOT_CAPABILITY_METRICS.with_borrow_mut(|counts| {
            let key = RootCapabilityMetricStorageKey {
                capability,
                event_type,
                outcome,
                proof_mode,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// record_envelope
    ///
    /// Record envelope-stage metric events with envelope-specific outcomes.
    pub fn record_envelope(
        capability: RootCapabilityMetricKey,
        outcome: RootCapabilityMetricOutcome,
        proof_mode: RootCapabilityMetricProofMode,
    ) {
        Self::record(
            capability,
            RootCapabilityMetricEventType::Envelope,
            outcome,
            proof_mode,
        );
    }

    /// record_proof
    ///
    /// Record proof-stage metric events with proof-specific outcomes.
    pub fn record_proof(
        capability: RootCapabilityMetricKey,
        outcome: RootCapabilityMetricOutcome,
        proof_mode: RootCapabilityMetricProofMode,
    ) {
        Self::record(
            capability,
            RootCapabilityMetricEventType::Proof,
            outcome,
            proof_mode,
        );
    }

    /// record_authorization
    ///
    /// Record authorization-stage metric events with authorization-specific outcomes.
    pub fn record_authorization(
        capability: RootCapabilityMetricKey,
        outcome: RootCapabilityMetricOutcome,
    ) {
        Self::record(
            capability,
            RootCapabilityMetricEventType::Authorization,
            outcome,
            RootCapabilityMetricProofMode::Unspecified,
        );
    }

    /// record_replay
    ///
    /// Record replay-stage metric events with replay-specific outcomes.
    pub fn record_replay(
        capability: RootCapabilityMetricKey,
        outcome: RootCapabilityMetricOutcome,
    ) {
        Self::record(
            capability,
            RootCapabilityMetricEventType::Replay,
            outcome,
            RootCapabilityMetricProofMode::Unspecified,
        );
    }

    /// record_execution
    ///
    /// Record execution-stage metric events with execution-specific outcomes.
    pub fn record_execution(
        capability: RootCapabilityMetricKey,
        outcome: RootCapabilityMetricOutcome,
    ) {
        Self::record(
            capability,
            RootCapabilityMetricEventType::Execution,
            outcome,
            RootCapabilityMetricProofMode::Unspecified,
        );
    }

    #[must_use]
    pub fn snapshot() -> Vec<(
        RootCapabilityMetricKey,
        RootCapabilityMetricEventType,
        RootCapabilityMetricOutcome,
        RootCapabilityMetricProofMode,
        u64,
    )> {
        ROOT_CAPABILITY_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .map(|(key, count)| {
                (
                    key.capability,
                    key.event_type,
                    key.outcome,
                    key.proof_mode,
                    count,
                )
            })
            .collect()
    }

    #[cfg(test)]
    pub fn reset() {
        ROOT_CAPABILITY_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn snapshot_map() -> HashMap<
        (
            RootCapabilityMetricKey,
            RootCapabilityMetricEventType,
            RootCapabilityMetricOutcome,
            RootCapabilityMetricProofMode,
        ),
        u64,
    > {
        RootCapabilityMetrics::snapshot()
            .into_iter()
            .map(|(capability, event_type, outcome, proof_mode, count)| {
                ((capability, event_type, outcome, proof_mode), count)
            })
            .collect()
    }

    #[test]
    fn root_capability_metrics_start_empty() {
        RootCapabilityMetrics::reset();

        let snapshot = RootCapabilityMetrics::snapshot();
        assert!(snapshot.is_empty());
    }

    #[test]
    fn record_increments_for_same_key_and_event() {
        RootCapabilityMetrics::reset();

        RootCapabilityMetrics::record_authorization(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
        );
        RootCapabilityMetrics::record_authorization(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEventType::Authorization,
                RootCapabilityMetricOutcome::Accepted,
                RootCapabilityMetricProofMode::Unspecified,
            )),
            Some(&2)
        );
    }

    #[test]
    fn metrics_are_partitioned_by_capability_event_and_proof_mode() {
        RootCapabilityMetrics::reset();

        RootCapabilityMetrics::record_authorization(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
        );
        RootCapabilityMetrics::record_authorization(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Denied,
        );
        RootCapabilityMetrics::record_proof(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
            RootCapabilityMetricProofMode::RoleAttestation,
        );
        RootCapabilityMetrics::record_proof(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
            RootCapabilityMetricProofMode::DelegatedGrant,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEventType::Authorization,
                RootCapabilityMetricOutcome::Accepted,
                RootCapabilityMetricProofMode::Unspecified,
            )),
            Some(&1)
        );
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEventType::Authorization,
                RootCapabilityMetricOutcome::Denied,
                RootCapabilityMetricProofMode::Unspecified,
            )),
            Some(&1)
        );
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEventType::Proof,
                RootCapabilityMetricOutcome::Accepted,
                RootCapabilityMetricProofMode::RoleAttestation,
            )),
            Some(&1)
        );
        assert_eq!(
            map.get(&(
                RootCapabilityMetricKey::Provision,
                RootCapabilityMetricEventType::Proof,
                RootCapabilityMetricOutcome::Accepted,
                RootCapabilityMetricProofMode::DelegatedGrant,
            )),
            Some(&1)
        );
    }
}
