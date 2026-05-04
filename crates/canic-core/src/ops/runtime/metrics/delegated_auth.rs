use crate::cdk::types::Principal;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static DELEGATED_AUTH_AUTHORITIES: RefCell<HashMap<Principal, u64>> =
        RefCell::new(HashMap::new());
    static DELEGATED_AUTH_EVENTS: RefCell<HashMap<DelegatedAuthMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// DelegatedAuthMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DelegatedAuthMetricOperation {
    VerifyToken,
}

impl DelegatedAuthMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::VerifyToken => "verify_token",
        }
    }
}

///
/// DelegatedAuthMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DelegatedAuthMetricOutcome {
    Completed,
    Failed,
    Started,
}

impl DelegatedAuthMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Started => "started",
        }
    }
}

///
/// DelegatedAuthMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DelegatedAuthMetricReason {
    Audience,
    AudienceNotSubset,
    Canonical,
    CertAudienceRejected,
    CertExpired,
    CertHashMismatch,
    CertNotYetValid,
    CertPolicy,
    Disabled,
    InvalidState,
    IssuerShardPidMismatch,
    LocalRoleHashMismatch,
    MissingLocalRole,
    Ok,
    RootKey,
    RootSignatureInvalid,
    RootSignatureUnavailable,
    ScopeRejected,
    ShardKeyBinding,
    ShardSignatureInvalid,
    ShardSignatureUnavailable,
    TokenAudienceRejected,
    TokenExpired,
    TokenInvalidWindow,
    TokenIssuedBeforeCert,
    TokenNotYetValid,
    TokenOutlivesCert,
    TokenTtlExceeded,
}

impl DelegatedAuthMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Audience => "audience",
            Self::AudienceNotSubset => "audience_not_subset",
            Self::Canonical => "canonical",
            Self::CertAudienceRejected => "cert_audience_rejected",
            Self::CertExpired => "cert_expired",
            Self::CertHashMismatch => "cert_hash_mismatch",
            Self::CertNotYetValid => "cert_not_yet_valid",
            Self::CertPolicy => "cert_policy",
            Self::Disabled => "disabled",
            Self::InvalidState => "invalid_state",
            Self::IssuerShardPidMismatch => "issuer_shard_pid_mismatch",
            Self::LocalRoleHashMismatch => "local_role_hash_mismatch",
            Self::MissingLocalRole => "missing_local_role",
            Self::Ok => "ok",
            Self::RootKey => "root_key",
            Self::RootSignatureInvalid => "root_signature_invalid",
            Self::RootSignatureUnavailable => "root_signature_unavailable",
            Self::ScopeRejected => "scope_rejected",
            Self::ShardKeyBinding => "shard_key_binding",
            Self::ShardSignatureInvalid => "shard_signature_invalid",
            Self::ShardSignatureUnavailable => "shard_signature_unavailable",
            Self::TokenAudienceRejected => "token_audience_rejected",
            Self::TokenExpired => "token_expired",
            Self::TokenInvalidWindow => "token_invalid_window",
            Self::TokenIssuedBeforeCert => "token_issued_before_cert",
            Self::TokenNotYetValid => "token_not_yet_valid",
            Self::TokenOutlivesCert => "token_outlives_cert",
            Self::TokenTtlExceeded => "token_ttl_exceeded",
        }
    }
}

///
/// DelegatedAuthMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct DelegatedAuthMetricKey {
    pub operation: DelegatedAuthMetricOperation,
    pub outcome: DelegatedAuthMetricOutcome,
    pub reason: DelegatedAuthMetricReason,
}

///
/// DelegatedAuthMetrics
/// Records verified delegation authorities by signer principal.
/// Cardinality is bounded by configured signer authorities, not request callers.
///

pub struct DelegatedAuthMetrics;

impl DelegatedAuthMetrics {
    /// Record one successful delegated-authority verification.
    pub fn record_authority(authority: Principal) {
        DELEGATED_AUTH_AUTHORITIES.with_borrow_mut(|counts| {
            let entry = counts.entry(authority).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Record that delegated-token verification started.
    pub fn record_verify_started() {
        Self::record(
            DelegatedAuthMetricOperation::VerifyToken,
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that delegated-token verification completed successfully.
    pub fn record_verify_completed() {
        Self::record(
            DelegatedAuthMetricOperation::VerifyToken,
            DelegatedAuthMetricOutcome::Completed,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that delegated-token verification failed with a bounded reason.
    pub fn record_verify_failed(reason: DelegatedAuthMetricReason) {
        Self::record(
            DelegatedAuthMetricOperation::VerifyToken,
            DelegatedAuthMetricOutcome::Failed,
            reason,
        );
    }

    /// Record one delegated-auth verification event.
    pub fn record(
        operation: DelegatedAuthMetricOperation,
        outcome: DelegatedAuthMetricOutcome,
        reason: DelegatedAuthMetricReason,
    ) {
        DELEGATED_AUTH_EVENTS.with_borrow_mut(|counts| {
            let key = DelegatedAuthMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current delegated-auth authority table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(Principal, u64)> {
        DELEGATED_AUTH_AUTHORITIES
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Snapshot the current delegated-auth event table as stable rows.
    #[must_use]
    pub fn event_snapshot() -> Vec<(DelegatedAuthMetricKey, u64)> {
        DELEGATED_AUTH_EVENTS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all delegated-auth metrics.
    #[cfg(test)]
    pub fn reset() {
        DELEGATED_AUTH_AUTHORITIES.with_borrow_mut(HashMap::clear);
        DELEGATED_AUTH_EVENTS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn snapshot_map() -> HashMap<Principal, u64> {
        DelegatedAuthMetrics::snapshot().into_iter().collect()
    }

    // Convert event snapshots into a map for concise count assertions.
    fn event_snapshot_map() -> HashMap<DelegatedAuthMetricKey, u64> {
        DelegatedAuthMetrics::event_snapshot().into_iter().collect()
    }

    #[test]
    fn record_authority_increments() {
        DelegatedAuthMetrics::reset();

        let pid = p(1);
        DelegatedAuthMetrics::record_authority(pid);
        DelegatedAuthMetrics::record_authority(pid);

        let map = snapshot_map();
        assert_eq!(map.get(&pid), Some(&2));
    }

    #[test]
    fn record_verify_outcomes_increment() {
        DelegatedAuthMetrics::reset();

        DelegatedAuthMetrics::record_verify_started();
        DelegatedAuthMetrics::record_verify_completed();
        DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::TokenExpired);
        DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::TokenExpired);

        let map = event_snapshot_map();
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::VerifyToken,
                outcome: DelegatedAuthMetricOutcome::Started,
                reason: DelegatedAuthMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::VerifyToken,
                outcome: DelegatedAuthMetricOutcome::Completed,
                reason: DelegatedAuthMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::VerifyToken,
                outcome: DelegatedAuthMetricOutcome::Failed,
                reason: DelegatedAuthMetricReason::TokenExpired,
            }),
            Some(&2)
        );
    }
}
