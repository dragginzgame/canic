//! Module: ops::runtime::metrics::delegated_auth
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the delegated_auth family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

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
/// Delegated auth metric operation dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DelegatedAuthMetricOperation {
    PrepareIssuerProof,
    PrepareRootProof,
    RenewalAttempt,
    RenewalSweep,
    VerifyToken,
}

impl DelegatedAuthMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::PrepareIssuerProof => "prepare_issuer_proof",
            Self::PrepareRootProof => "prepare_root_proof",
            Self::RenewalAttempt => "renewal_attempt",
            Self::RenewalSweep => "renewal_sweep",
            Self::VerifyToken => "verify_token",
        }
    }
}

///
/// DelegatedAuthMetricOutcome
///
/// Delegated auth metric outcome dimension used by public metrics projection.
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
/// Bounded delegated auth reason dimension used by public metrics projection.
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
    GrantsNotSubset,
    InvalidState,
    IssuerPidMismatch,
    IssuerProofInvalid,
    IssuerProofPrepareFailed,
    IssuerProofUnavailable,
    MissingLocalRole,
    Ok,
    RootKey,
    RootProofInvalid,
    RootProofPrepareFailed,
    ScopeRejected,
    TokenAudienceRejected,
    TokenExpired,
    TokenGrantRejected,
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
            Self::GrantsNotSubset => "grants_not_subset",
            Self::InvalidState => "invalid_state",
            Self::IssuerPidMismatch => "issuer_pid_mismatch",
            Self::IssuerProofInvalid => "issuer_proof_invalid",
            Self::IssuerProofPrepareFailed => "issuer_proof_prepare_failed",
            Self::IssuerProofUnavailable => "issuer_proof_unavailable",
            Self::MissingLocalRole => "missing_local_role",
            Self::Ok => "ok",
            Self::RootKey => "root_key",
            Self::RootProofPrepareFailed => "root_proof_prepare_failed",
            Self::RootProofInvalid => "root_proof_invalid",
            Self::ScopeRejected => "scope_rejected",
            Self::TokenAudienceRejected => "token_audience_rejected",
            Self::TokenExpired => "token_expired",
            Self::TokenGrantRejected => "token_grant_rejected",
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
/// Composite key for one low-cardinality delegated auth counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct DelegatedAuthMetricKey {
    pub operation: DelegatedAuthMetricOperation,
    pub outcome: DelegatedAuthMetricOutcome,
    pub reason: DelegatedAuthMetricReason,
}

///
/// DelegatedAuthMetrics
///
/// Records verified delegation authorities by issuer/root authority principal.
/// Cardinality is bounded by configured auth authorities, not request callers.
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

    /// Record that root proof preparation started.
    #[cfg_attr(
        all(not(test), not(feature = "auth-root-canister-sig-create")),
        expect(dead_code)
    )]
    pub fn record_root_proof_prepare_started() {
        Self::record(
            DelegatedAuthMetricOperation::PrepareRootProof,
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that root proof preparation completed successfully.
    #[cfg_attr(
        all(not(test), not(feature = "auth-root-canister-sig-create")),
        expect(dead_code)
    )]
    pub fn record_root_proof_prepare_completed() {
        Self::record(
            DelegatedAuthMetricOperation::PrepareRootProof,
            DelegatedAuthMetricOutcome::Completed,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that root proof preparation failed.
    #[cfg_attr(
        all(not(test), feature = "auth-root-canister-sig-create"),
        expect(dead_code)
    )]
    pub fn record_root_proof_prepare_failed() {
        Self::record(
            DelegatedAuthMetricOperation::PrepareRootProof,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::RootProofPrepareFailed,
        );
    }

    /// Record that issuer proof preparation started.
    #[cfg_attr(
        all(not(test), not(feature = "auth-issuer-canister-sig-create")),
        expect(dead_code)
    )]
    pub fn record_issuer_proof_prepare_started() {
        Self::record(
            DelegatedAuthMetricOperation::PrepareIssuerProof,
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that issuer proof preparation completed successfully.
    #[cfg_attr(
        all(not(test), not(feature = "auth-issuer-canister-sig-create")),
        expect(dead_code)
    )]
    pub fn record_issuer_proof_prepare_completed() {
        Self::record(
            DelegatedAuthMetricOperation::PrepareIssuerProof,
            DelegatedAuthMetricOutcome::Completed,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that issuer proof preparation failed.
    #[cfg_attr(
        all(not(test), feature = "auth-issuer-canister-sig-create"),
        expect(dead_code)
    )]
    pub fn record_issuer_proof_prepare_failed() {
        Self::record(
            DelegatedAuthMetricOperation::PrepareIssuerProof,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::IssuerProofPrepareFailed,
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

    /// Record that root-managed renewal sweep preparation started.
    pub fn record_renewal_sweep_started() {
        Self::record(
            DelegatedAuthMetricOperation::RenewalSweep,
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that root-managed renewal sweep preparation completed.
    pub fn record_renewal_sweep_completed() {
        Self::record(
            DelegatedAuthMetricOperation::RenewalSweep,
            DelegatedAuthMetricOutcome::Completed,
            DelegatedAuthMetricReason::Ok,
        );
    }

    /// Record that root-managed renewal sweep preparation failed.
    pub fn record_renewal_sweep_failed() {
        Self::record(
            DelegatedAuthMetricOperation::RenewalSweep,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::RootProofPrepareFailed,
        );
    }

    /// Record a scheduled issuer-level root renewal attempt lifecycle event.
    pub fn record_renewal_attempt(
        outcome: DelegatedAuthMetricOutcome,
        reason: DelegatedAuthMetricReason,
    ) {
        Self::record(
            DelegatedAuthMetricOperation::RenewalAttempt,
            outcome,
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

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

    fn assert_event_count(
        map: &HashMap<DelegatedAuthMetricKey, u64>,
        operation: DelegatedAuthMetricOperation,
        outcome: DelegatedAuthMetricOutcome,
        reason: DelegatedAuthMetricReason,
        count: u64,
    ) {
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation,
                outcome,
                reason,
            }),
            Some(&count)
        );
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

        DelegatedAuthMetrics::record_root_proof_prepare_started();
        DelegatedAuthMetrics::record_root_proof_prepare_completed();
        DelegatedAuthMetrics::record_root_proof_prepare_failed();
        DelegatedAuthMetrics::record_issuer_proof_prepare_started();
        DelegatedAuthMetrics::record_issuer_proof_prepare_completed();
        DelegatedAuthMetrics::record_issuer_proof_prepare_failed();
        DelegatedAuthMetrics::record_verify_started();
        DelegatedAuthMetrics::record_verify_completed();
        DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::TokenExpired);
        DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::TokenExpired);

        let map = event_snapshot_map();
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::PrepareRootProof,
                outcome: DelegatedAuthMetricOutcome::Started,
                reason: DelegatedAuthMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::PrepareRootProof,
                outcome: DelegatedAuthMetricOutcome::Completed,
                reason: DelegatedAuthMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::PrepareRootProof,
                outcome: DelegatedAuthMetricOutcome::Failed,
                reason: DelegatedAuthMetricReason::RootProofPrepareFailed,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::PrepareIssuerProof,
                outcome: DelegatedAuthMetricOutcome::Started,
                reason: DelegatedAuthMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::PrepareIssuerProof,
                outcome: DelegatedAuthMetricOutcome::Completed,
                reason: DelegatedAuthMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DelegatedAuthMetricKey {
                operation: DelegatedAuthMetricOperation::PrepareIssuerProof,
                outcome: DelegatedAuthMetricOutcome::Failed,
                reason: DelegatedAuthMetricReason::IssuerProofPrepareFailed,
            }),
            Some(&1)
        );
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

    #[test]
    fn record_renewal_outcomes_increment() {
        DelegatedAuthMetrics::reset();

        DelegatedAuthMetrics::record_renewal_sweep_started();
        DelegatedAuthMetrics::record_renewal_sweep_completed();
        DelegatedAuthMetrics::record_renewal_sweep_failed();

        let map = event_snapshot_map();
        assert_event_count(
            &map,
            DelegatedAuthMetricOperation::RenewalSweep,
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
            1,
        );
        assert_event_count(
            &map,
            DelegatedAuthMetricOperation::RenewalSweep,
            DelegatedAuthMetricOutcome::Completed,
            DelegatedAuthMetricReason::Ok,
            1,
        );
        assert_event_count(
            &map,
            DelegatedAuthMetricOperation::RenewalSweep,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::RootProofPrepareFailed,
            1,
        );
    }

    #[test]
    fn record_renewal_attempt_outcomes_increment() {
        DelegatedAuthMetrics::reset();

        DelegatedAuthMetrics::record_renewal_attempt(
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        );
        DelegatedAuthMetrics::record_renewal_attempt(
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::RootProofPrepareFailed,
        );

        let map = event_snapshot_map();
        assert_event_count(
            &map,
            DelegatedAuthMetricOperation::RenewalAttempt,
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
            1,
        );
        assert_event_count(
            &map,
            DelegatedAuthMetricOperation::RenewalAttempt,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::RootProofPrepareFailed,
            1,
        );
    }
}
