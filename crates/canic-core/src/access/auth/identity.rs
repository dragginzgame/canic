use super::{
    AuthenticatedIdentitySource, DelegatedSessionSubjectRejection, ResolvedAuthenticatedIdentity,
};
use crate::{
    cdk::types::Principal,
    ops::{
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_session_fallback_invalid_subject, record_session_fallback_raw_caller,
        },
        storage::{auth::AuthStateOps, registry::subnet::SubnetRegistryOps},
    },
};

/// resolve_authenticated_identity
///
/// Resolve transport caller and authenticated subject for user auth checks.
#[must_use]
pub(super) fn resolve_authenticated_identity(
    transport_caller: Principal,
) -> ResolvedAuthenticatedIdentity {
    resolve_authenticated_identity_at(transport_caller, crate::ops::ic::IcOps::now_secs())
}

pub(super) fn resolve_authenticated_identity_at(
    transport_caller: Principal,
    now_secs: u64,
) -> ResolvedAuthenticatedIdentity {
    if let Some(session) = AuthStateOps::delegated_session(transport_caller, now_secs) {
        if validate_delegated_session_subject(session.delegated_pid).is_ok() {
            return ResolvedAuthenticatedIdentity {
                transport_caller,
                authenticated_subject: session.delegated_pid,
                identity_source: AuthenticatedIdentitySource::DelegatedSession,
            };
        }

        AuthStateOps::clear_delegated_session(transport_caller);
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
pub(super) fn validate_delegated_session_subject(
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
    Some(crate::cdk::api::canister_self())
}

#[cfg(not(target_arch = "wasm32"))]
const fn try_canister_self() -> Option<Principal> {
    None
}
