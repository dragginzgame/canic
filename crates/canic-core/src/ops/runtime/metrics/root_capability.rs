use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ROOT_CAPABILITY_METRICS: RefCell<HashMap<RootCapabilityMetricTuple, u64>> =
        RefCell::new(HashMap::new());
}

///
/// RootCapabilityMetricKey
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootCapabilityMetricKey {
    Provision,
    Upgrade,
    MintCycles,
    IssueDelegation,
    IssueRoleAttestation,
}

impl RootCapabilityMetricKey {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Provision => "Provision",
            Self::Upgrade => "Upgrade",
            Self::MintCycles => "MintCycles",
            Self::IssueDelegation => "IssueDelegation",
            Self::IssueRoleAttestation => "IssueRoleAttestation",
        }
    }
}

///
/// RootCapabilityMetricEventType
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootCapabilityMetricEventType {
    Envelope,
    Proof,
    Authorization,
    Replay,
    Execution,
}

impl RootCapabilityMetricEventType {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Envelope => "Envelope",
            Self::Proof => "Proof",
            Self::Authorization => "Authorization",
            Self::Replay => "Replay",
            Self::Execution => "Execution",
        }
    }
}

///
/// RootCapabilityMetricOutcome
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootCapabilityMetricOutcome {
    Accepted,
    Rejected,
    Denied,
    DuplicateSame,
    DuplicateConflict,
    Expired,
    TtlExceeded,
    Success,
    Error,
}

impl RootCapabilityMetricOutcome {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Accepted => "Accepted",
            Self::Rejected => "Rejected",
            Self::Denied => "Denied",
            Self::DuplicateSame => "DuplicateSame",
            Self::DuplicateConflict => "DuplicateConflict",
            Self::Expired => "Expired",
            Self::TtlExceeded => "TtlExceeded",
            Self::Success => "Success",
            Self::Error => "Error",
        }
    }
}

///
/// RootCapabilityMetricProofMode
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootCapabilityMetricProofMode {
    Unspecified,
    Structural,
    RoleAttestation,
    DelegatedGrant,
}

impl RootCapabilityMetricProofMode {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Unspecified => "Unspecified",
            Self::Structural => "Structural",
            Self::RoleAttestation => "RoleAttestation",
            Self::DelegatedGrant => "DelegatedGrant",
        }
    }
}

///
/// RootCapabilityMetricTuple
///

pub type RootCapabilityMetricTuple = (
    RootCapabilityMetricKey,
    RootCapabilityMetricEventType,
    RootCapabilityMetricOutcome,
    RootCapabilityMetricProofMode,
);

///
/// RootCapabilityMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct RootCapabilityMetricsSnapshot {
    pub entries: Vec<(
        RootCapabilityMetricKey,
        RootCapabilityMetricEventType,
        RootCapabilityMetricOutcome,
        RootCapabilityMetricProofMode,
        u64,
    )>,
}

///
/// RootCapabilityMetrics
///

pub struct RootCapabilityMetrics;

impl RootCapabilityMetrics {
    /// record_metric
    ///
    /// Record a root capability metric point using event type, outcome, and proof mode dimensions.
    pub fn record_metric(
        capability: RootCapabilityMetricKey,
        event_type: RootCapabilityMetricEventType,
        outcome: RootCapabilityMetricOutcome,
        proof_mode: RootCapabilityMetricProofMode,
    ) {
        ROOT_CAPABILITY_METRICS.with_borrow_mut(|counts| {
            let entry = counts
                .entry((capability, event_type, outcome, proof_mode))
                .or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// record
    ///
    /// Record a root capability metric point for event types without proof-mode context.
    pub fn record(
        capability: RootCapabilityMetricKey,
        event_type: RootCapabilityMetricEventType,
        outcome: RootCapabilityMetricOutcome,
    ) {
        Self::record_metric(
            capability,
            event_type,
            outcome,
            RootCapabilityMetricProofMode::Unspecified,
        );
    }

    #[must_use]
    pub fn snapshot() -> RootCapabilityMetricsSnapshot {
        let entries = ROOT_CAPABILITY_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .map(|((capability, event_type, outcome, proof_mode), count)| {
                (capability, event_type, outcome, proof_mode, count)
            })
            .collect();

        RootCapabilityMetricsSnapshot { entries }
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

    fn snapshot_map() -> HashMap<RootCapabilityMetricTuple, u64> {
        RootCapabilityMetrics::snapshot()
            .entries
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
        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn record_increments_for_same_key_and_event() {
        RootCapabilityMetrics::reset();

        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEventType::Authorization,
            RootCapabilityMetricOutcome::Accepted,
        );
        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEventType::Authorization,
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

        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEventType::Authorization,
            RootCapabilityMetricOutcome::Accepted,
        );
        RootCapabilityMetrics::record(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEventType::Authorization,
            RootCapabilityMetricOutcome::Denied,
        );
        RootCapabilityMetrics::record_metric(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEventType::Proof,
            RootCapabilityMetricOutcome::Accepted,
            RootCapabilityMetricProofMode::RoleAttestation,
        );
        RootCapabilityMetrics::record_metric(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricEventType::Proof,
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
