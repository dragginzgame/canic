//! Auth access checks.
//!
//! This bucket includes:
//! - caller identity checks (controller/whitelist)
//! - topology checks (parent/child/root/same canister)
//! - registry-based role checks
//! - delegated token verification
//!
//! Security invariants for delegated tokens:
//! - Delegated tokens are only valid if their proof matches a verifier-local keyed delegation proof.
//! - Delegation rotation may retain multiple proofs concurrently until older tokens age out.
//! - All temporal validation (iat/exp/now) is enforced before access is granted.
//! - Endpoint-required scopes are enforced against delegated token claims.

mod identity;
mod predicates;
mod token;

use crate::{
    access::AccessError,
    cdk::types::Principal,
    ids::CanisterRole,
    ops::{
        auth::VerifiedDelegatedToken, runtime::env::EnvOps,
        storage::registry::subnet::SubnetRegistryOps,
    },
};
use std::fmt;

pub type Role = CanisterRole;

///
/// AuthenticatedIdentitySource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticatedIdentitySource {
    RawCaller,
    DelegatedSession,
}

///
/// ResolvedAuthenticatedIdentity
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

pub(crate) async fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
) -> Result<VerifiedDelegatedToken, AccessError> {
    token::delegated_token_verified(authenticated_subject, required_scope).await
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

/// Require that the caller appears in the active whitelist (IC deployments).
/// No-op on local builds; enforces whitelist on IC.
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

// -----------------------------------------------------------------------------
// Registry predicates
// -----------------------------------------------------------------------------

/// Require that the caller is registered with the expected canister role.
pub async fn has_role(caller: Principal, role: Role) -> Result<(), AccessError> {
    predicates::has_role(caller, role).await
}

/// Ensure the caller matches the app directory entry recorded for `role`.
/// Require that the caller is registered as a canister on this subnet.
pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), AccessError> {
    predicates::is_registered_to_subnet(caller).await
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
        assert!(err.to_string().contains("does not match caller"));
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
        assert!(err.to_string().contains("missing required scope"));
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
        crate::ops::storage::auth::DelegationStateOps::clear_delegated_session(wallet);
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
        crate::ops::storage::auth::DelegationStateOps::upsert_delegated_session(
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

        crate::ops::storage::auth::DelegationStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_when_session_expired() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(6);
        let delegated = p(5);
        crate::ops::storage::auth::DelegationStateOps::upsert_delegated_session(
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

        crate::ops::storage::auth::DelegationStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_after_clear() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(4);
        let delegated = p(3);
        crate::ops::storage::auth::DelegationStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: delegated,
                issued_at: 50,
                expires_at: 500,
                bootstrap_token_fingerprint: None,
            },
            50,
        );
        crate::ops::storage::auth::DelegationStateOps::clear_delegated_session(wallet);

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
        crate::ops::storage::auth::DelegationStateOps::upsert_delegated_session(
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
            crate::ops::storage::auth::DelegationStateOps::delegated_session(wallet, 20).is_none(),
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
