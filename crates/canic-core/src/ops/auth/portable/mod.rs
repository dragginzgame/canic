//! Module: ops::auth::portable
//!
//! Responsibility: compose protected Canic enrollment with the complete Auth engine.
//! Boundary: workflow supplies the current gate; submitted claims are lookup hints only.

pub mod runtime;

use crate::{
    config::schema::LocalApplicationAuthorizationConfig,
    model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    ops::{
        auth::{AuthOps, delegated::root_proof},
        config::ConfigOps,
        storage::auth::{
            LocalApplicationAuthorizationStateOps,
            application_sessions::{
                ApplicationSessionStore, PersistentReceiverError, PersistentReceiverState,
            },
        },
    },
    storage::stable::auth::{LocalApplicationAuthorizationState, PortableReceiverFloorsRecord},
};
use canic_contracts::dto::portable_enrollment::{
    PortableEnrollmentSnapshot, PortableIssuerEnrollment, PortableIssuerKey,
};
use ic_auth::{
    portable::{PortableAuthority, PortableClaimsContext, ValidatedGrants},
    portable_credential::PortableVerificationContext,
    session::{
        RequiredScopes, Session, SessionAdmissionContext, SessionContext, SessionError,
        SessionPolicy, SessionStore,
    },
    session_request::{SessionRequest, SessionTtlLimit, SessionTtlPolicy},
};
use ic_auth_protocol_types::{
    AudienceId, AuthRole, CanonicalId, PortableCredential, PortableMechanism, Principal,
};
use ic_auth_wire::{SessionInput, SessionView};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Preserve the original host/engine failure for local diagnostics.
#[derive(Debug, Error)]
pub enum PortableSessionError {
    #[error("portable receiver authority is unavailable")]
    Unavailable,
    #[error("portable receiver production clock skew is unqualified")]
    UnqualifiedClock,
    #[error("portable session is missing")]
    Missing,
    #[error("portable session expired")]
    Expired,
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Receiver(#[from] PersistentReceiverError),
    #[error(transparent)]
    Enrollment(#[from] crate::ops::auth::enrollment::PortableEnrollmentError),
    #[error(transparent)]
    Runtime(#[from] crate::InternalError),
    #[error(transparent)]
    Wire(#[from] ic_wire::Error),
}

impl PortableSessionError {
    const fn engine(&self) -> Option<&SessionError> {
        match self {
            Self::Session(error) | Self::Receiver(PersistentReceiverError::Session(error)) => {
                Some(error)
            }
            _ => None,
        }
    }
    pub fn admission_refusal(&self) -> ic_auth_wire::model::AdmissionRefusal {
        use ic_auth_protocol_types::AdmissionRefusal as R;
        self.engine()
            .map_or_else(
                || match self {
                    Self::Wire(_) => R::InvalidCredential,
                    Self::Unavailable | Self::UnqualifiedClock | Self::Enrollment(_) => {
                        R::AuthorityUnavailable
                    }
                    Self::Expired | Self::Missing => R::Expired,
                    _ => R::Unavailable,
                },
                SessionError::admission_refusal,
            )
            .into()
    }
    pub fn read_refusal(&self) -> ic_auth_wire::model::SessionReadRefusal {
        use ic_auth_protocol_types::SessionReadRefusal as R;
        self.engine()
            .map_or_else(
                || match self {
                    Self::Missing | Self::Expired => R::MissingOrExpired,
                    Self::Unavailable | Self::UnqualifiedClock | Self::Enrollment(_) => {
                        R::AuthorityUnavailable
                    }
                    _ => R::Unavailable,
                },
                SessionError::read_refusal,
            )
            .into()
    }
    pub const fn denial(&self) -> crate::domain::policy::pure::auth::application_authorization::LocalApplicationAuthorizationDenial{
        use crate::domain::policy::pure::auth::application_authorization::LocalApplicationAuthorizationDenial as D;
        match self {
            Self::Missing => D::MissingSession,
            Self::Expired => D::Expired,
            _ => match self.engine() {
                Some(SessionError::ScopeRejected) => D::MissingScope,
                Some(SessionError::StaleAuthority | SessionError::Revocation(_)) => {
                    D::StaleAuthority
                }
                _ => D::AuthorityUnavailable,
            },
        }
    }
    pub const fn logout_unavailable(&self) -> ic_auth_protocol_types::LogoutResult {
        use ic_auth_protocol_types::{LogoutResult as R, LogoutUnavailable as U};
        R::Unavailable(match self {
            Self::Unavailable => U::ResetPending,
            Self::Receiver(PersistentReceiverError::Clock) => U::Clock,
            _ => U::Storage,
        })
    }
}

/// Current receiver-local policy and a freshly authenticated Root snapshot.
pub struct PortableReceiverContext<'a> {
    pub config: &'a LocalApplicationAuthorizationConfig,
    pub identity: &'a LocalApplicationAuthoritySnapshot,
    pub snapshot: &'a PortableEnrollmentSnapshot,
    pub caller: Principal,
    pub now_ns: u64,
}

struct Selector {
    issuer: Principal,
    mechanism: PortableMechanism,
    binding: [u8; 32],
}

fn entry<'a>(
    snapshot: &'a PortableEnrollmentSnapshot,
    selector: &Selector,
) -> Result<&'a PortableIssuerEnrollment, PortableSessionError> {
    snapshot
        .entries
        .iter()
        .find(|entry| {
            if entry.issuer != selector.issuer {
                return false;
            }
            match (&entry.key, selector.mechanism) {
                (
                    PortableIssuerKey::Certified { certificate_hash },
                    PortableMechanism::Certified,
                ) => *certificate_hash == selector.binding,
                (PortableIssuerKey::Ed25519 { public_key }, PortableMechanism::Ed25519) => {
                    <[u8; 32]>::from(Sha256::digest(public_key)) == selector.binding
                }
                _ => false,
            }
        })
        .ok_or(PortableSessionError::Unavailable)
}

fn grants(entry: &PortableIssuerEnrollment) -> Result<ValidatedGrants, SessionError> {
    crate::ops::auth::enrollment::validation::grants(&entry.grants)
        .map_err(|_| SessionError::InvalidPolicyOrRecord)
}

const fn audience(context: &PortableReceiverContext<'_>) -> AudienceId {
    let fleet = context.identity.fleet();
    AudienceId {
        canonical_network_id: CanonicalId::from_bytes(*fleet.canonical_network_id.as_bytes()),
        fleet_id: CanonicalId::from_bytes(*fleet.fleet_id.as_bytes()),
    }
}

fn authority<'a>(
    context: &PortableReceiverContext<'_>,
    entry: &PortableIssuerEnrollment,
    audience: &'a AudienceId,
    grants: &'a ValidatedGrants,
    floors: PortableReceiverFloorsRecord,
) -> PortableAuthority<'a> {
    let (mechanism, binding) = match &entry.key {
        PortableIssuerKey::Certified { certificate_hash } => {
            (PortableMechanism::Certified, *certificate_hash)
        }
        PortableIssuerKey::Ed25519 { public_key } => (
            PortableMechanism::Ed25519,
            Sha256::digest(public_key).into(),
        ),
    };
    PortableAuthority {
        audience,
        issuer: entry.issuer,
        mechanism,
        key_binding_id: binding,
        grants,
        caller: context.caller,
        now_ns: context.now_ns,
        authority_not_before_ns: entry.not_before_ns,
        authority_expires_at_ns: entry
            .expires_at_ns
            .min(entry.deployment_approval_expires_at_ns),
        enrollment_expires_at_ns: context.snapshot.expires_at_ns,
        min_login_started_at_ns: floors.min_login_started_at_ns,
    }
}

fn policy() -> SessionPolicy {
    SessionPolicy {
        generation: LocalApplicationAuthorizationStateOps::application_authority_generation(),
        enabled: true,
        // Approved issuers authenticate accounts; application resource policy remains separate.
        subject_admissible: true,
    }
}

fn session_view(session: &Session) -> SessionView {
    SessionView {
        caller: session.caller(),
        issuer: session.issuer(),
        subject: session.subject(),
        expires_at_ns: session.expires_at_ns(),
    }
}

/// Admit one exact credential through the selected complete verifier and atomic backend.
pub fn admit(
    owner: &mut PersistentReceiverState,
    context: PortableReceiverContext<'_>,
    input: SessionInput,
) -> Result<SessionView, PortableSessionError> {
    let profile = context
        .config
        .receiver
        .as_ref()
        .ok_or(PortableSessionError::Unavailable)?;
    let floors = LocalApplicationAuthorizationState::receiver_floors()
        .ok_or(PortableSessionError::Unavailable)?;
    let credential: PortableCredential = input.credential.try_into()?;
    let hint = ic_auth::portable::decode_claims(&credential.claims).map_err(SessionError::from)?;
    let enrollment = entry(
        context.snapshot,
        &Selector {
            issuer: hint.issuer,
            mechanism: hint.mechanism,
            binding: hint.key_binding_id,
        },
    )?;
    let request = SessionRequest::new(
        input
            .scopes
            .into_vec()
            .into_iter()
            .map(ic_wire::Text::into_string)
            .collect(),
        input.ttl_secs,
    )
    .map_err(SessionError::from)?;
    if request.scopes().iter().any(|scope| {
        !context
            .config
            .allowed_scopes
            .iter()
            .any(|allowed| allowed == scope.as_str())
    }) {
        return Err(SessionError::ScopeRejected.into());
    }
    let role: AuthRole = context
        .identity
        .role()
        .as_str()
        .parse()
        .map_err(|_| SessionError::InvalidPolicyOrRecord)?;
    let audience = audience(&context);
    let grants = grants(enrollment)?;
    let claims = PortableClaimsContext {
        authority: authority(&context, enrollment, &audience, &grants, floors),
        min_proof_issued_at_ns: floors.min_proof_issued_at_ns,
        max_login_ttl_ns: profile.max_login_ttl_ns,
        max_proof_ttl_ns: profile.max_proof_ttl_ns,
    };
    let ttl = SessionTtlPolicy::new(
        context.config.default_session_ttl_secs,
        context.config.maximum_session_ttl_secs,
    )
    .map_err(SessionError::from)?;
    establish_enrolled(
        owner,
        &context,
        enrollment,
        Admission {
            credential: &credential,
            request,
            claims,
            role: &role,
            ttl,
        },
    )
}

struct Admission<'a> {
    credential: &'a PortableCredential,
    request: SessionRequest,
    claims: PortableClaimsContext<'a>,
    role: &'a AuthRole,
    ttl: SessionTtlPolicy,
}

fn establish_enrolled(
    owner: &mut PersistentReceiverState,
    context: &PortableReceiverContext<'_>,
    enrollment: &PortableIssuerEnrollment,
    admission: Admission<'_>,
) -> Result<SessionView, PortableSessionError> {
    let Admission {
        credential,
        request,
        claims,
        role,
        ttl,
    } = admission;
    let profile = context
        .config
        .receiver
        .as_ref()
        .ok_or(PortableSessionError::Unavailable)?;
    let result = match &enrollment.key {
        PortableIssuerKey::Ed25519 { public_key } => {
            let key = ic_auth::portable_signature::PreparedIssuerKey::new(*public_key)
                .map_err(|e| SessionError::Credential(e.into()))?;
            owner.run(context.now_ns, |store, ledger, budget, _| {
                ic_auth::session::establish_session(
                    store,
                    ledger,
                    budget,
                    credential,
                    request,
                    &SessionAdmissionContext {
                        verification: PortableVerificationContext::Ed25519 {
                            claims: &claims,
                            public_key: &key,
                        },
                        role,
                        ttl,
                    },
                    policy(),
                )
            })?
        }
        PortableIssuerKey::Certified { .. } => {
            let verifier = AuthOps::auth_proof_verifier_config()?;
            let root = verifier
                .chain_key_root
                .as_ref()
                .ok_or(PortableSessionError::Unavailable)?;
            if verifier.root_canister_id != context.snapshot.authority.into()
                || !root_proof::root_policy_permitted(root)
            {
                return Err(PortableSessionError::Unavailable);
            }
            let root_key = root_proof::project_root_policy(&root.policy);
            let max_ttl = ConfigOps::delegated_tokens_config()?
                .max_ttl_secs
                .unwrap_or(86_400)
                .checked_mul(1_000_000_000)
                .ok_or(PortableSessionError::Unavailable)?;
            let certified = ic_auth::portable_certified::PortableCertifiedContext {
                claims,
                root_key: &root_key,
                ic_root_public_key_raw: &verifier.ic_root_public_key_raw,
                limits: ic_auth::portable_certified::CertifiedProofLimits {
                    max_cert_ttl_ns: max_ttl,
                    max_token_ttl_ns: max_ttl,
                    max_certificate_age_ns: profile.max_certificate_age_ns,
                    max_variable_bytes: 64 * 1024,
                    max_issuer_signature_bytes: 64 * 1024,
                    max_witness_steps: 64,
                },
            };
            owner.run(context.now_ns, |store, ledger, budget, _| {
                ic_auth::session::establish_session(
                    store,
                    ledger,
                    budget,
                    credential,
                    request,
                    &SessionAdmissionContext {
                        verification: PortableVerificationContext::Certified(&certified),
                        role,
                        ttl,
                    },
                    policy(),
                )
            })?
        }
    }
    .receipt();
    Ok(SessionView {
        caller: result.caller(),
        issuer: result.issuer(),
        subject: result.subject(),
        expires_at_ns: result.expires_at_ns(),
    })
}

/// Authorize inside the current local snapshot; queries publish no stable writes.
pub fn read(
    owner: &mut PersistentReceiverState,
    context: PortableReceiverContext<'_>,
    required: Vec<String>,
    replicated: bool,
) -> Result<SessionView, PortableSessionError> {
    let floors = LocalApplicationAuthorizationState::receiver_floors()
        .ok_or(PortableSessionError::Unavailable)?;
    let mut found = false;
    let mut run = |store: &mut ApplicationSessionStore,
                   ledger: &mut ic_auth::portable::MemoryLoginRevocations| {
        let selector = store.read(|reader| {
            Ok(reader.session(context.caller)?.map(|s| Selector {
                issuer: s.issuer(),
                mechanism: s.mechanism(),
                binding: s.key_binding_id(),
            }))
        })?;
        found = selector.is_some();
        let selector = selector.ok_or(SessionError::MissingOrExpired)?;
        let enrollment =
            entry(context.snapshot, &selector).map_err(|_| SessionError::StaleAuthority)?;
        let role = context
            .identity
            .role()
            .as_str()
            .parse()
            .map_err(|_| SessionError::InvalidPolicyOrRecord)?;
        let audience = audience(&context);
        let grants = grants(enrollment)?;
        let mut authority = authority(&context, enrollment, &audience, &grants, floors);
        if matches!(enrollment.key, PortableIssuerKey::Certified { .. }) {
            let verifier =
                AuthOps::auth_proof_verifier_config().map_err(|_| SessionError::StaleAuthority)?;
            let root = verifier
                .chain_key_root
                .ok_or(SessionError::StaleAuthority)?;
            if verifier.root_canister_id != context.snapshot.authority.into()
                || !root_proof::root_policy_permitted(&root)
            {
                return Err(SessionError::StaleAuthority);
            }
            authority.authority_not_before_ns = authority
                .authority_not_before_ns
                .max(root.policy.valid_from_ns);
            authority.authority_expires_at_ns = authority
                .authority_expires_at_ns
                .min(root.policy.accept_until_ns);
        }
        let scopes = RequiredScopes::new(required.clone())?;
        let allowed = ic_auth::session::authorize_session(
            store,
            ledger,
            &SessionContext {
                authority,
                max_ttl: SessionTtlLimit::new(context.config.maximum_session_ttl_secs)?,
                role: &role,
                required_scopes: &scopes,
            },
            policy(),
            session_view,
        )?;
        if required
            .iter()
            .any(|scope| !context.config.allowed_scopes.contains(scope))
        {
            return Err(SessionError::ScopeRejected);
        }
        Ok(allowed)
    };
    let result = if replicated {
        owner.run(context.now_ns, |store, ledger, _, _| run(store, ledger))
    } else {
        owner.view(context.now_ns, run)
    };
    match result {
        Err(PersistentReceiverError::Session(SessionError::MissingOrExpired)) if found => {
            Err(PortableSessionError::Expired)
        }
        Err(PersistentReceiverError::Session(SessionError::MissingOrExpired)) => {
            Err(PortableSessionError::Missing)
        }
        other => other.map_err(Into::into),
    }
}

pub fn logout(
    owner: &mut PersistentReceiverState,
    caller: Principal,
    now_ns: u64,
) -> Result<ic_auth_protocol_types::LogoutResult, PortableSessionError> {
    owner
        .run(now_ns, |store, ledger, _, unknown| {
            Ok(ic_auth::session::logout_session(
                store, ledger, unknown, caller, now_ns,
            ))
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod tests;
