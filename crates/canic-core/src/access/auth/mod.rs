//! Module: access::auth
//!
//! Responsibility: resolve endpoint caller identity and enforce auth predicates.
//! Does not own: endpoint response mapping, operation replay safety, or storage schema.
//! Boundary: access expressions call auth predicates before endpoint workflow execution.

mod identity;
mod predicates;
mod token;

use crate::{
    access::AccessError,
    cdk::types::Principal,
    ids::EndpointCallKind,
    ops::{runtime::env::EnvOps, storage::registry::subnet::SubnetRegistryOps},
};
use std::fmt;

///
/// AuthenticatedIdentitySource
///
/// Source used to resolve the authenticated endpoint subject.
/// Owned by access auth and stored in access evaluation context.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedIdentitySource {
    RawCaller,
    DelegatedSession,
}

///
/// ResolvedAuthenticatedIdentity
///
/// Transport caller plus resolved authenticated subject for access evaluation.
/// Owned by access auth and returned to endpoint access plumbing.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedAuthenticatedIdentity {
    pub transport_caller: Principal,
    pub authenticated_subject: Principal,
    pub identity_source: AuthenticatedIdentitySource,
}

///
/// DelegatedSessionSubjectRejection
///
/// Reason a delegated session subject cannot be accepted as a user identity.
/// Owned by access auth and used to reject infrastructure principals.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedSessionSubjectRejection {
    Anonymous,
    ManagementCanister,
    LocalCanister,
    RootCanister,
    ParentCanister,
    SubnetCanister,
    PrimeRootCanister,
    RegisteredCanister,
}

impl fmt::Display for DelegatedSessionSubjectRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::Anonymous => "anonymous principals are not allowed",
            Self::ManagementCanister => "management canister principal is not allowed",
            Self::LocalCanister => "current canister principal is not allowed",
            Self::RootCanister => "root canister principal is not allowed",
            Self::ParentCanister => "parent canister principal is not allowed",
            Self::SubnetCanister => "subnet principal is not allowed",
            Self::PrimeRootCanister => "prime root principal is not allowed",
            Self::RegisteredCanister => "subnet-registered canister principal is not allowed",
        };
        f.write_str(reason)
    }
}

/// resolve_authenticated_identity
///
/// Resolve transport caller and authenticated subject for user auth checks.
#[must_use]
pub fn resolve_authenticated_identity(
    transport_caller: Principal,
) -> ResolvedAuthenticatedIdentity {
    identity::resolve_authenticated_identity(transport_caller)
}

#[cfg(test)]
pub(crate) fn resolve_authenticated_identity_at(
    transport_caller: Principal,
    now_secs: u64,
) -> ResolvedAuthenticatedIdentity {
    identity::resolve_authenticated_identity_at(transport_caller, now_secs)
}

/// validate_delegated_session_subject
///
/// Reject obvious canister and infrastructure identities for delegated user sessions.
pub fn validate_delegated_session_subject(
    subject: Principal,
) -> Result<(), DelegatedSessionSubjectRejection> {
    identity::validate_delegated_session_subject(subject)
}

pub(crate) fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
    call_kind: EndpointCallKind,
) -> Result<Principal, AccessError> {
    token::delegated_token_verified(authenticated_subject, required_scope, call_kind)
}

#[cfg(test)]
fn enforce_subject_binding(sub: Principal, caller: Principal) -> Result<(), AccessError> {
    token::enforce_subject_binding(sub, caller)
}

#[cfg(test)]
fn enforce_required_scope(
    required_scope: Option<&str>,
    token_scopes: &[String],
) -> Result<(), AccessError> {
    token::enforce_required_scope(required_scope, token_scopes)
}

// -----------------------------------------------------------------------------
// Caller & topology predicates
// -----------------------------------------------------------------------------

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
pub async fn is_controller(caller: Principal) -> Result<(), AccessError> {
    predicates::is_controller(caller).await
}

/// Require that the caller appears in the configured whitelist.
/// Missing whitelist configuration fails closed.
pub async fn is_whitelisted(caller: Principal) -> Result<(), AccessError> {
    predicates::is_whitelisted(caller).await
}

/// Require that the caller is a direct child of the current canister.
pub async fn is_child(caller: Principal) -> Result<(), AccessError> {
    predicates::is_child(caller).await
}

/// Require that the caller is the configured parent canister.
pub async fn is_parent(caller: Principal) -> Result<(), AccessError> {
    predicates::is_parent(caller).await
}

/// Require that the caller equals the configured root canister.
pub async fn is_root(caller: Principal) -> Result<(), AccessError> {
    predicates::is_root(caller).await
}

/// Require that the caller is the currently executing canister.
pub async fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    predicates::is_same_canister(caller).await
}

/// Require that the caller is registered as a canister on this subnet.
pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), AccessError> {
    predicates::is_registered_to_subnet(caller).await
}

/// Require that the caller is an enabled root-managed renewal provisioner.
pub async fn is_delegation_renewal_provisioner(caller: Principal) -> Result<(), AccessError> {
    predicates::is_delegation_renewal_provisioner(caller).await
}

fn dependency_unavailable(detail: &str) -> AccessError {
    AccessError::Denied(format!("access dependency unavailable: {detail}"))
}

fn non_root_subnet_registry_predicate_denial() -> AccessError {
    AccessError::Denied(
        "authentication error: illegal access to subnet registry predicate from non-root canister"
            .to_string(),
    )
}

fn caller_not_registered_denial(caller: Principal) -> AccessError {
    let root = EnvOps::root_pid().map_or_else(|_| "unavailable".to_string(), |pid| pid.to_string());
    let registry_count = SubnetRegistryOps::data().entries.len();
    AccessError::Denied(format!(
        "authentication error: caller '{caller}' is not registered on the subnet registry \
         (root='{root}', registry_entries={registry_count}); verify caller root routing and \
         canic_subnet_registry state"
    ))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{AccessMetricKind, cap},
        ops::runtime::metrics::access::AccessMetrics,
        test::seams,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn auth_session_metric_count(predicate: &str) -> u64 {
        AccessMetrics::snapshot()
            .entries
            .into_iter()
            .find_map(|(key, count)| {
                if key.endpoint == "auth_session"
                    && key.kind == AccessMetricKind::Auth
                    && key.predicate == predicate
                {
                    Some(count)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    #[test]
    fn subject_binding_allows_matching_subject_and_caller() {
        let sub = p(1);
        let caller = p(1);
        assert!(enforce_subject_binding(sub, caller).is_ok());
    }

    #[test]
    fn subject_binding_rejects_mismatched_subject_and_caller() {
        let sub = p(1);
        let caller = p(2);
        let err = enforce_subject_binding(sub, caller).expect_err("expected subject mismatch");
        assert!(matches!(err, AccessError::Denied(_)));
    }

    #[test]
    fn required_scope_allows_when_scope_present() {
        let scopes = vec![cap::READ.to_string(), cap::VERIFY.to_string()];
        assert!(enforce_required_scope(Some(cap::VERIFY), &scopes).is_ok());
    }

    #[test]
    fn required_scope_rejects_when_scope_missing() {
        let scopes = vec![cap::READ.to_string()];
        let err = enforce_required_scope(Some(cap::VERIFY), &scopes).expect_err("expected denial");
        assert!(matches!(err, AccessError::Denied(_)));
    }

    #[test]
    fn required_scope_none_is_allowed() {
        let scopes = vec![cap::READ.to_string()];
        assert!(enforce_required_scope(None, &scopes).is_ok());
    }

    #[test]
    fn resolve_authenticated_identity_defaults_to_wallet_when_no_override_exists() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(9);
        crate::ops::storage::auth::AuthStateOps::clear_delegated_session(wallet);
        let resolved = resolve_authenticated_identity(wallet);
        assert_eq!(resolved.authenticated_subject, wallet);
        assert_eq!(
            auth_session_metric_count("session_fallback_raw_caller"),
            1,
            "missing delegated session should record raw-caller fallback"
        );
    }

    #[test]
    fn resolve_authenticated_identity_prefers_active_delegated_session() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(8);
        let delegated = p(7);
        crate::ops::storage::auth::AuthStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: delegated,
                issued_at: 100,
                expires_at: 200,
                bootstrap_token_fingerprint: None,
            },
            100,
        );

        let resolved = resolve_authenticated_identity_at(wallet, 150);
        assert_eq!(resolved.transport_caller, wallet);
        assert_eq!(resolved.authenticated_subject, delegated);
        assert_eq!(
            resolved.identity_source,
            AuthenticatedIdentitySource::DelegatedSession
        );
        assert_eq!(
            auth_session_metric_count("session_fallback_raw_caller"),
            0,
            "active delegated session should not fallback to raw caller"
        );

        crate::ops::storage::auth::AuthStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_when_session_expired() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(6);
        let delegated = p(5);
        crate::ops::storage::auth::AuthStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: delegated,
                issued_at: 100,
                expires_at: 120,
                bootstrap_token_fingerprint: None,
            },
            100,
        );

        let resolved = resolve_authenticated_identity_at(wallet, 121);
        assert_eq!(resolved.authenticated_subject, wallet);
        assert_eq!(
            resolved.identity_source,
            AuthenticatedIdentitySource::RawCaller
        );
        assert_eq!(
            auth_session_metric_count("session_fallback_raw_caller"),
            1,
            "expired delegated session should fallback to raw caller"
        );

        crate::ops::storage::auth::AuthStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_at_session_expiry_boundary() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(16);
        let delegated = p(15);
        crate::ops::storage::auth::AuthStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: delegated,
                issued_at: 100,
                expires_at: 120,
                bootstrap_token_fingerprint: None,
            },
            100,
        );

        let resolved = resolve_authenticated_identity_at(wallet, 120);
        assert_eq!(resolved.authenticated_subject, wallet);
        assert_eq!(
            resolved.identity_source,
            AuthenticatedIdentitySource::RawCaller
        );
        assert_eq!(
            auth_session_metric_count("session_fallback_raw_caller"),
            1,
            "delegated session expiry must match token expiry boundary"
        );

        crate::ops::storage::auth::AuthStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_after_clear() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(4);
        let delegated = p(3);
        crate::ops::storage::auth::AuthStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: delegated,
                issued_at: 50,
                expires_at: 500,
                bootstrap_token_fingerprint: None,
            },
            50,
        );
        crate::ops::storage::auth::AuthStateOps::clear_delegated_session(wallet);

        let resolved = resolve_authenticated_identity_at(wallet, 100);
        assert_eq!(resolved.authenticated_subject, wallet);
        assert_eq!(
            resolved.identity_source,
            AuthenticatedIdentitySource::RawCaller
        );
        assert_eq!(auth_session_metric_count("session_fallback_raw_caller"), 1);
    }

    #[test]
    fn resolve_authenticated_identity_records_invalid_subject_fallback() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(23);
        crate::ops::storage::auth::AuthStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: Principal::management_canister(),
                issued_at: 10,
                expires_at: 100,
                bootstrap_token_fingerprint: None,
            },
            10,
        );

        let resolved = resolve_authenticated_identity_at(wallet, 20);
        assert_eq!(resolved.authenticated_subject, wallet);
        assert_eq!(
            resolved.identity_source,
            AuthenticatedIdentitySource::RawCaller
        );
        assert_eq!(
            auth_session_metric_count("session_fallback_invalid_subject"),
            1
        );
        assert_eq!(auth_session_metric_count("session_fallback_raw_caller"), 1);
        assert!(
            crate::ops::storage::auth::AuthStateOps::delegated_session(wallet, 20).is_none(),
            "invalid delegated session should be cleared"
        );
    }

    #[test]
    fn validate_delegated_session_subject_rejects_anonymous() {
        let _guard = seams::lock();
        let err = validate_delegated_session_subject(Principal::anonymous())
            .expect_err("anonymous must be rejected");
        assert_eq!(err, DelegatedSessionSubjectRejection::Anonymous);
    }

    #[test]
    fn validate_delegated_session_subject_rejects_management_canister() {
        let _guard = seams::lock();
        let err = validate_delegated_session_subject(Principal::management_canister())
            .expect_err("management canister must be rejected");
        assert_eq!(err, DelegatedSessionSubjectRejection::ManagementCanister);
    }
}
