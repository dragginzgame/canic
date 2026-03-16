//! Auth access checks.
//!
//! This bucket includes:
//! - caller identity checks (controller/whitelist)
//! - topology checks (parent/child/root/same canister)
//! - registry-based role checks
//! - delegated token verification
//!
//! Security invariants for delegated tokens:
//! - Delegated tokens are only valid if their proof matches the currently stored delegation proof.
//! - Delegation rotation invalidates all previously issued delegated tokens.
//! - All temporal validation (iat/exp/now) is enforced before access is granted.
//! - Endpoint-required scopes are enforced against delegated token claims.

use crate::{
    access::AccessError,
    cdk::{
        api::{canister_self, is_controller as caller_is_controller, msg_arg_data},
        candid::de::IDLDeserialize,
        types::Principal,
    },
    config::Config,
    dto::auth::DelegatedToken,
    ids::CanisterRole,
    ops::{
        auth::{DelegatedTokenOps, VerifiedDelegatedToken},
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_session_fallback_invalid_subject, record_session_fallback_raw_caller,
        },
        storage::{
            auth::DelegationStateOps, children::CanisterChildrenOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
};
use std::fmt;

const MAX_INGRESS_BYTES: usize = 64 * 1024; // 64 KiB

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

///
/// CallerBoundToken
///
/// Verified delegated token that has passed caller-subject binding.
struct CallerBoundToken {
    verified: VerifiedDelegatedToken,
}

impl CallerBoundToken {
    /// bind_to_caller
    ///
    /// Enforce subject binding and return a caller-bound token wrapper.
    fn bind_to_caller(
        verified: VerifiedDelegatedToken,
        caller: Principal,
    ) -> Result<Self, AccessError> {
        enforce_subject_binding(verified.claims.sub, caller)?;
        Ok(Self { verified })
    }

    /// scopes
    ///
    /// Borrow token scopes after caller binding has been enforced.
    fn scopes(&self) -> &[String] {
        &self.verified.claims.scopes
    }

    /// into_verified
    ///
    /// Unwrap the verified delegated token for downstream consumers.
    fn into_verified(self) -> VerifiedDelegatedToken {
        self.verified
    }
}

/// resolve_authenticated_identity
///
/// Resolve transport caller and authenticated subject for user auth checks.
#[must_use]
pub fn resolve_authenticated_identity(
    transport_caller: Principal,
) -> ResolvedAuthenticatedIdentity {
    resolve_authenticated_identity_at(transport_caller, IcOps::now_secs())
}

pub(crate) fn resolve_authenticated_identity_at(
    transport_caller: Principal,
    now_secs: u64,
) -> ResolvedAuthenticatedIdentity {
    if let Some(session) = DelegationStateOps::delegated_session(transport_caller, now_secs) {
        if validate_delegated_session_subject(session.delegated_pid).is_ok() {
            return ResolvedAuthenticatedIdentity {
                transport_caller,
                authenticated_subject: session.delegated_pid,
                identity_source: AuthenticatedIdentitySource::DelegatedSession,
            };
        }

        DelegationStateOps::clear_delegated_session(transport_caller);
        record_session_fallback_invalid_subject();
    }

    record_session_fallback_raw_caller();
    ResolvedAuthenticatedIdentity {
        transport_caller,
        authenticated_subject: transport_caller,
        identity_source: AuthenticatedIdentitySource::RawCaller,
    }
}

/// validate_delegated_session_subject
///
/// Reject obvious canister and infrastructure identities for delegated user sessions.
pub fn validate_delegated_session_subject(
    subject: Principal,
) -> Result<(), DelegatedSessionSubjectRejection> {
    if subject == Principal::anonymous() {
        return Err(DelegatedSessionSubjectRejection::Anonymous);
    }

    if subject == Principal::management_canister() {
        return Err(DelegatedSessionSubjectRejection::ManagementCanister);
    }

    if try_canister_self().is_some_and(|pid| pid == subject) {
        return Err(DelegatedSessionSubjectRejection::LocalCanister);
    }

    let env = EnvOps::snapshot();
    if env.root_pid.is_some_and(|pid| pid == subject) {
        return Err(DelegatedSessionSubjectRejection::RootCanister);
    }
    if env.parent_pid.is_some_and(|pid| pid == subject) {
        return Err(DelegatedSessionSubjectRejection::ParentCanister);
    }
    if env.subnet_pid.is_some_and(|pid| pid == subject) {
        return Err(DelegatedSessionSubjectRejection::SubnetCanister);
    }
    if env.prime_root_pid.is_some_and(|pid| pid == subject) {
        return Err(DelegatedSessionSubjectRejection::PrimeRootCanister);
    }
    if SubnetRegistryOps::is_registered(subject) {
        return Err(DelegatedSessionSubjectRejection::RegisteredCanister);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[expect(clippy::unnecessary_wraps)]
fn try_canister_self() -> Option<Principal> {
    Some(IcOps::canister_self())
}

#[cfg(not(target_arch = "wasm32"))]
const fn try_canister_self() -> Option<Principal> {
    None
}

pub(crate) async fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
) -> Result<VerifiedDelegatedToken, AccessError> {
    let token = delegated_token_from_args()?;

    let authority_pid =
        EnvOps::root_pid().map_err(|_| dependency_unavailable("root pid unavailable"))?;

    let now_secs = IcOps::now_secs();
    let self_pid = IcOps::canister_self();

    verify_token(
        token,
        authenticated_subject,
        authority_pid,
        now_secs,
        self_pid,
        required_scope,
    )
    .await
}

/// Verify a delegated token against the configured authority.
#[expect(clippy::unused_async)]
async fn verify_token(
    token: DelegatedToken,
    caller: Principal,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
    required_scope: Option<&str>,
) -> Result<VerifiedDelegatedToken, AccessError> {
    let verified = DelegatedTokenOps::verify_token(&token, authority_pid, now_secs, self_pid)
        .map_err(|err| AccessError::Denied(err.to_string()))?;

    let caller_bound = CallerBoundToken::bind_to_caller(verified, caller)?;
    enforce_required_scope(required_scope, caller_bound.scopes())?;

    Ok(caller_bound.into_verified())
}

fn enforce_subject_binding(sub: Principal, caller: Principal) -> Result<(), AccessError> {
    if sub == caller {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "delegated token subject '{sub}' does not match caller '{caller}'"
        )))
    }
}

fn enforce_required_scope(
    required_scope: Option<&str>,
    token_scopes: &[String],
) -> Result<(), AccessError> {
    let Some(required_scope) = required_scope else {
        return Ok(());
    };

    if token_scopes.iter().any(|scope| scope == required_scope) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "delegated token missing required scope '{required_scope}'"
        )))
    }
}

// -----------------------------------------------------------------------------
// Caller & topology predicates
// -----------------------------------------------------------------------------

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
#[expect(clippy::unused_async)]
pub async fn is_controller(caller: Principal) -> Result<(), AccessError> {
    if caller_is_controller(&caller) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not a controller of this canister"
        )))
    }
}

/// Require that the caller appears in the active whitelist (IC deployments).
/// No-op on local builds; enforces whitelist on IC.
#[expect(clippy::unused_async)]
pub async fn is_whitelisted(caller: Principal) -> Result<(), AccessError> {
    let cfg = Config::try_get().ok_or_else(|| dependency_unavailable("config not initialized"))?;

    if !cfg.is_whitelisted(&caller) {
        return Err(AccessError::Denied(format!(
            "caller '{caller}' is not on the whitelist"
        )));
    }

    Ok(())
}

/// Require that the caller is a direct child of the current canister.
#[expect(clippy::unused_async)]
pub async fn is_child(caller: Principal) -> Result<(), AccessError> {
    if CanisterChildrenOps::contains_pid(&caller) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not a child of this canister"
        )))
    }
}

/// Require that the caller is the configured parent canister.
#[expect(clippy::unused_async)]
pub async fn is_parent(caller: Principal) -> Result<(), AccessError> {
    let snapshot = EnvOps::snapshot();
    let parent_pid = snapshot
        .parent_pid
        .ok_or_else(|| dependency_unavailable("parent pid unavailable"))?;

    if parent_pid == caller {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not the parent of this canister"
        )))
    }
}

/// Require that the caller equals the configured root canister.
#[expect(clippy::unused_async)]
pub async fn is_root(caller: Principal) -> Result<(), AccessError> {
    let root_pid =
        EnvOps::root_pid().map_err(|_| dependency_unavailable("root pid unavailable"))?;

    if caller == root_pid {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not root"
        )))
    }
}

/// Require that the caller is the currently executing canister.
#[expect(clippy::unused_async)]
pub async fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    if caller == canister_self() {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "caller '{caller}' is not the current canister"
        )))
    }
}

// -----------------------------------------------------------------------------
// Registry predicates
// -----------------------------------------------------------------------------

/// Require that the caller is registered with the expected canister role.
#[expect(clippy::unused_async)]
pub async fn has_role(caller: Principal, role: Role) -> Result<(), AccessError> {
    if !EnvOps::is_root() {
        return Err(non_root_subnet_registry_predicate_denial());
    }

    let record =
        SubnetRegistryOps::get(caller).ok_or_else(|| caller_not_registered_denial(caller))?;

    if record.role == role {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "authentication error: caller '{caller}' does not have role '{role}'"
        )))
    }
}

/// Ensure the caller matches the app directory entry recorded for `role`.
/// Require that the caller is registered as a canister on this subnet.
#[expect(clippy::unused_async)]
pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), AccessError> {
    if !EnvOps::is_root() {
        return Err(non_root_subnet_registry_predicate_denial());
    }

    if SubnetRegistryOps::is_registered(caller) {
        Ok(())
    } else {
        Err(caller_not_registered_denial(caller))
    }
}

fn delegated_token_from_args() -> Result<DelegatedToken, AccessError> {
    let bytes = msg_arg_data();

    if bytes.len() > MAX_INGRESS_BYTES {
        return Err(AccessError::Denied(
            "delegated token payload exceeds size limit".to_string(),
        ));
    }

    let mut decoder = IDLDeserialize::new(&bytes)
        .map_err(|err| AccessError::Denied(format!("failed to decode ingress arguments: {err}")))?;

    decoder.get_value::<DelegatedToken>().map_err(|err| {
        AccessError::Denied(format!(
            "failed to decode delegated token as first argument: {err}"
        ))
    })
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
        DelegationStateOps::clear_delegated_session(wallet);
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
        DelegationStateOps::upsert_delegated_session(
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

        DelegationStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_when_session_expired() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(6);
        let delegated = p(5);
        DelegationStateOps::upsert_delegated_session(
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

        DelegationStateOps::clear_delegated_session(wallet);
    }

    #[test]
    fn resolve_authenticated_identity_falls_back_after_clear() {
        let _guard = seams::lock();
        AccessMetrics::reset();
        let wallet = p(4);
        let delegated = p(3);
        DelegationStateOps::upsert_delegated_session(
            crate::ops::storage::auth::DelegatedSession {
                wallet_pid: wallet,
                delegated_pid: delegated,
                issued_at: 50,
                expires_at: 500,
                bootstrap_token_fingerprint: None,
            },
            50,
        );
        DelegationStateOps::clear_delegated_session(wallet);

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
        DelegationStateOps::upsert_delegated_session(
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
            DelegationStateOps::delegated_session(wallet, 20).is_none(),
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
