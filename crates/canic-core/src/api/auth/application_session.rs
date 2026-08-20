//! Module: api::auth::application_session
//!
//! Responsibility: adapt managed application-session commands and caller-self status.
//! Does not own: proof cryptography, canonical persistence, or role-surface selection.
//! Boundary: cfg-pruned endpoint macros call this adapter with ambient caller/time authority.

use super::AuthApi;
use crate::{
    InternalError,
    access::auth::validate_application_subject,
    config::schema::LocalApplicationAuthorizationConfig,
    diagnostics::codes,
    domain::policy::pure::auth::application_authorization::{
        ApplicationCapacityLimit, ApplicationProofEligibilityError,
        ApplicationSessionAdmissionError, ApplicationSessionTtlError,
        resolve_application_session_expiry,
    },
    dto::{
        auth::{
            ApplicationSessionAuditEntry, ApplicationSessionAuditResponse,
            ApplicationSessionCommandResponse, ApplicationSessionPolicyView,
            ApplicationSessionRequest, ApplicationSessionStatus,
            ApplicationSessionVerifierPolicyView, ApplicationSessionView,
            InactiveApplicationSession,
        },
        error::Error,
        page::{Page, PageRequest},
    },
    model::auth::application_authorization::{
        ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthoritySnapshot,
        LocalApplicationSession, MAX_APPLICATION_PROOF_LIFETIME_NS,
    },
    ops::{
        auth::{
            AuthOps, VerifyDelegatedTokenRuntimeInput,
            application_authorization::LocalApplicationAuthorizationAuthority,
        },
        ic::IcOps,
        runtime::metrics::auth::{
            AuthMetricReason, record_application_session_clear, record_application_session_created,
            record_application_session_establishment_started,
            record_application_session_expired_observation, record_application_session_idempotent,
            record_application_session_rejected, record_application_session_replaced,
        },
        storage::auth::{
            AuthStateOps,
            application_sessions::{ApplicationReplayResolution, ApplicationSessionStateError},
        },
    },
    workflow::{
        auth::application_sessions::{
            ApplicationSessionEstablishInput, ApplicationSessionEstablishResult,
            ApplicationSessionWorkflow, ApplicationSessionWorkflowError,
        },
        runtime::intent::IntentCleanupWorkflow,
    },
};

const NS_PER_SEC: u64 = 1_000_000_000;

impl AuthApi {
    /// Establish one caller-bound scoped session from a current delegated proof.
    pub fn establish_application_session(
        request: ApplicationSessionRequest,
    ) -> Result<ApplicationSessionCommandResponse, Error> {
        record_application_session_establishment_started();
        let result = Self::establish_application_session_inner(request);
        if let Err(error) = &result {
            record_application_session_rejected(establishment_rejection_reason(*error));
        }
        result
    }

    fn establish_application_session_inner(
        request: ApplicationSessionRequest,
    ) -> Result<ApplicationSessionCommandResponse, Error> {
        let caller = IcOps::msg_caller();
        require_application_caller(caller)?;
        let authority = application_session_authority()?;
        let now_ns = IcOps::now_nanos();
        let requested_ttl_secs = request.requested_ttl_secs;
        let requested_scopes = canonical_requested_scopes(request.requested_scopes)?;
        require_configured_scopes(&authority.config, &requested_scopes)?;
        let expires_at_ns = session_expiry(now_ns, requested_ttl_secs, &authority.config)?;
        let proof_fingerprint =
            AuthOps::delegated_token_claims_fingerprint(&request.delegated_token)
                .map_err(Error::from)?;
        let request_hash = ApplicationSessionWorkflow::establishment_request_hash(
            &requested_scopes,
            requested_ttl_secs,
        );

        match ApplicationSessionWorkflow::resolve_retry(
            proof_fingerprint,
            caller,
            request_hash,
            now_ns,
        )
        .map_err(map_state_error)?
        {
            ApplicationReplayResolution::ExactActive(session) => {
                require_current_session(&session, &authority.snapshot, now_ns)?;
                record_application_session_idempotent();
                IntentCleanupWorkflow::reconcile_after_terminal();
                return Ok(ApplicationSessionCommandResponse::Established(
                    session_view(&session),
                ));
            }
            ApplicationReplayResolution::Conflict => {
                return Err(Error::from_registered(codes::STATE_CONFLICT));
            }
            ApplicationReplayResolution::Absent => {}
        }

        let max_ttl_ns = AuthOps::delegated_token_max_ttl_ns().map_err(Error::from)?;
        let verified = AuthOps::verify_token(VerifyDelegatedTokenRuntimeInput {
            token: &request.delegated_token,
            caller,
            max_cert_ttl_ns: max_ttl_ns,
            max_token_ttl_ns: max_ttl_ns,
            required_scopes: &[],
            now_ns,
        })
        .map_err(Error::from)?;
        if verified.fleet() != authority.snapshot.fleet()
            || verified.role() != authority.snapshot.role()
        {
            return Err(Error::from_registered(codes::AUTHORITY_CONFLICT));
        }

        let established =
            ApplicationSessionWorkflow::establish_verified(ApplicationSessionEstablishInput {
                authority: verified,
                requested_scopes,
                authority_generation: authority.snapshot.generation(),
                established_at_ns: now_ns,
                expires_at_ns,
                establishment_request_hash: request_hash,
            })
            .map_err(map_workflow_error)?;
        let session = match established {
            ApplicationSessionEstablishResult::Created(session) => {
                record_application_session_created();
                session
            }
            ApplicationSessionEstablishResult::ExactRetry(session) => {
                record_application_session_idempotent();
                session
            }
            ApplicationSessionEstablishResult::Replaced(session) => {
                record_application_session_replaced();
                session
            }
        };
        IntentCleanupWorkflow::reconcile_after_terminal();
        Ok(ApplicationSessionCommandResponse::Established(
            session_view(&session),
        ))
    }

    /// Remove only the current caller's retained session and keep replay tombstones.
    pub fn clear_application_session() -> Result<ApplicationSessionCommandResponse, Error> {
        let caller = IcOps::msg_caller();
        require_application_caller(caller)?;
        application_session_authority()?;
        let removed = AuthStateOps::clear_application_session(caller).map_err(map_state_error)?;
        record_application_session_clear(removed);
        IntentCleanupWorkflow::reconcile_after_terminal();
        Ok(ApplicationSessionCommandResponse::Cleared)
    }

    /// Return the current caller's read-only application-session classification.
    pub fn application_session_status() -> Result<ApplicationSessionStatus, Error> {
        let caller = IcOps::msg_caller();
        require_application_caller(caller)?;
        let authority = application_session_authority()?;
        let Some(session) = AuthStateOps::application_session(caller).map_err(map_state_error)?
        else {
            return Ok(ApplicationSessionStatus::Inactive(
                InactiveApplicationSession::Missing,
            ));
        };
        let now_ns = IcOps::now_nanos();
        let inactive = classify_inactive_session(&session, &authority.snapshot, now_ns);
        if matches!(inactive, Some(InactiveApplicationSession::Expired { .. })) {
            record_application_session_expired_observation();
        }
        Ok(match inactive {
            Some(reason) => ApplicationSessionStatus::Inactive(reason),
            None => ApplicationSessionStatus::Active(session_view(&session)),
        })
    }

    /// Return one root-authorized bounded audit page of retained application sessions.
    pub fn application_session_audit(
        page: PageRequest,
    ) -> Result<ApplicationSessionAuditResponse, Error> {
        let authority = application_session_authority()?;
        let verifier = AuthOps::auth_proof_verifier_config().map_err(Error::from)?;
        let sessions = AuthStateOps::application_session_page(page.offset, page.limit)
            .map_err(map_state_error)?;
        let now_ns = IcOps::now_nanos();
        let entries = sessions
            .entries
            .into_iter()
            .map(|session| ApplicationSessionAuditEntry {
                transport_caller: session.transport_caller(),
                status: match classify_inactive_session(&session, &authority.snapshot, now_ns) {
                    Some(reason) => ApplicationSessionStatus::Inactive(reason),
                    None => ApplicationSessionStatus::Active(session_view(&session)),
                },
            })
            .collect();
        let minimum_accepted_registry_epoch = verifier
            .chain_key_root
            .as_ref()
            .map(|chain_key| chain_key.policy.min_accepted_registry_epoch);
        Ok(ApplicationSessionAuditResponse {
            policy: ApplicationSessionPolicyView {
                fleet: authority.snapshot.fleet(),
                role: authority.snapshot.role().clone(),
                authority_generation: authority.snapshot.generation(),
                allowed_scopes: authority.config.allowed_scopes,
                default_session_ttl_secs: authority.config.default_session_ttl_secs,
                maximum_session_ttl_secs: authority.config.maximum_session_ttl_secs,
                proof_lifetime_ceiling_ns: MAX_APPLICATION_PROOF_LIFETIME_NS,
                verifier: ApplicationSessionVerifierPolicyView {
                    root_canister_id: verifier.root_canister_id,
                    minimum_accepted_registry_epoch,
                },
            },
            sessions: Page {
                entries,
                total: u64::try_from(sessions.total).unwrap_or(u64::MAX),
            },
        })
    }
}

fn classify_inactive_session(
    session: &LocalApplicationSession,
    authority: &LocalApplicationAuthoritySnapshot,
    now_ns: u64,
) -> Option<InactiveApplicationSession> {
    if now_ns >= session.expires_at_ns() {
        Some(InactiveApplicationSession::Expired {
            expired_at_ns: session.expires_at_ns(),
        })
    } else if session.fleet() != authority.fleet() {
        Some(InactiveApplicationSession::StaleFleet)
    } else if session.role() != authority.role() {
        Some(InactiveApplicationSession::StaleRole)
    } else if session.authority_generation() != authority.generation() {
        Some(InactiveApplicationSession::StaleGeneration {
            session_generation: session.authority_generation(),
            current_generation: authority.generation(),
        })
    } else if validate_application_subject(session.authenticated_subject()).is_err() {
        Some(InactiveApplicationSession::InadmissibleSubject)
    } else {
        None
    }
}

fn require_application_caller(caller: crate::cdk::types::Principal) -> Result<(), Error> {
    validate_application_subject(caller)
        .map_err(|_| Error::from_registered(codes::AUTHORITY_UNAUTHORIZED))
}

fn application_session_authority() -> Result<LocalApplicationAuthorizationAuthority, Error> {
    AuthOps::local_application_authorization_authority()
        .map_err(Error::from)?
        .ok_or_else(|| Error::from_registered(codes::SECURITY_INACTIVE))
}

fn canonical_requested_scopes(scopes: Vec<String>) -> Result<CanonicalApplicationScopes, Error> {
    let scopes = scopes
        .into_iter()
        .map(ApplicationScope::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::from_registered(codes::REQUEST_INVALID))?;
    CanonicalApplicationScopes::for_session(scopes)
        .map_err(|_| Error::from_registered(codes::REQUEST_INVALID))
}

fn require_configured_scopes(
    config: &LocalApplicationAuthorizationConfig,
    requested: &CanonicalApplicationScopes,
) -> Result<(), Error> {
    if requested.as_slice().iter().all(|scope| {
        config
            .allowed_scopes
            .binary_search_by(|configured| configured.as_str().cmp(scope.as_str()))
            .is_ok()
    }) {
        Ok(())
    } else {
        Err(Error::from_registered(codes::AUTHORITY_CONFLICT))
    }
}

fn session_expiry(
    now_ns: u64,
    requested_ttl_secs: Option<u64>,
    config: &LocalApplicationAuthorizationConfig,
) -> Result<u64, Error> {
    let default_ttl_ns = config
        .default_session_ttl_secs
        .checked_mul(NS_PER_SEC)
        .ok_or_else(|| Error::from_registered(codes::TIME_CAPACITY))?;
    let maximum_ttl_ns = config
        .maximum_session_ttl_secs
        .checked_mul(NS_PER_SEC)
        .ok_or_else(|| Error::from_registered(codes::TIME_CAPACITY))?;
    let requested_ttl_ns = match requested_ttl_secs {
        Some(ttl) => Some(
            ttl.checked_mul(NS_PER_SEC)
                .ok_or_else(|| Error::from_registered(codes::TIME_CAPACITY))?,
        ),
        None => None,
    };
    resolve_application_session_expiry(now_ns, default_ttl_ns, maximum_ttl_ns, requested_ttl_ns)
        .map_err(map_ttl_error)
}

fn require_current_session(
    session: &LocalApplicationSession,
    authority: &LocalApplicationAuthoritySnapshot,
    now_ns: u64,
) -> Result<(), Error> {
    if classify_inactive_session(session, authority, now_ns).is_some() {
        return Err(Error::from_registered(codes::STATE_CONFLICT));
    }
    Ok(())
}

fn session_view(session: &LocalApplicationSession) -> ApplicationSessionView {
    ApplicationSessionView {
        authenticated_subject: session.authenticated_subject(),
        issuer: session.issuer(),
        scopes: session
            .scopes()
            .as_slice()
            .iter()
            .map(ToString::to_string)
            .collect(),
        established_at_ns: session.established_at_ns(),
        expires_at_ns: session.expires_at_ns(),
        authority_generation: session.authority_generation(),
    }
}

fn establishment_rejection_reason(error: Error) -> AuthMetricReason {
    let code = error.code();
    if code == codes::CAPACITY_LIMIT.raw_code() {
        AuthMetricReason::Capacity
    } else if code == codes::STATE_CONFLICT.raw_code() {
        AuthMetricReason::ReplayConflict
    } else if code == codes::AUTHORITY_CONFLICT.raw_code()
        || code == codes::AUTHORITY_UNAUTHORIZED.raw_code()
        || code == codes::SECURITY_INACTIVE.raw_code()
    {
        AuthMetricReason::AuthorityConflict
    } else if code == codes::REQUEST_INVALID.raw_code() || code == codes::TIME_CAPACITY.raw_code() {
        AuthMetricReason::InvalidRequest
    } else if code == codes::SECURITY_INVALID.raw_code()
        || code == codes::AUTH_CERT_EXPIRED.raw_code()
        || code == codes::AUTH_TOKEN_EXPIRED.raw_code()
    {
        AuthMetricReason::ProofInvalid
    } else {
        AuthMetricReason::StateUnavailable
    }
}

const fn map_ttl_error(error: ApplicationSessionTtlError) -> Error {
    match error {
        ApplicationSessionTtlError::RequestedZero
        | ApplicationSessionTtlError::RequestedExceedsMaximum => {
            Error::from_registered(codes::REQUEST_INVALID)
        }
        ApplicationSessionTtlError::DefaultZero
        | ApplicationSessionTtlError::MaximumZero
        | ApplicationSessionTtlError::DefaultExceedsMaximum
        | ApplicationSessionTtlError::MaximumExceedsHardLimit { .. } => {
            Error::from_registered(codes::CONFIGURATION_INVALID)
        }
        ApplicationSessionTtlError::ExpiryOverflow => Error::from_registered(codes::TIME_CAPACITY),
    }
}

fn map_workflow_error(error: ApplicationSessionWorkflowError) -> Error {
    match error {
        ApplicationSessionWorkflowError::ProofIneligible(error) => match error {
            ApplicationProofEligibilityError::LifetimeTooLong { .. } => {
                Error::from_registered(codes::TIME_CAPACITY)
            }
            ApplicationProofEligibilityError::CallerMismatch
            | ApplicationProofEligibilityError::InvalidWindow
            | ApplicationProofEligibilityError::NotCurrentlyValid => {
                Error::from_registered(codes::SECURITY_INVALID)
            }
        },
        ApplicationSessionWorkflowError::AdmissionDenied(error) => match error {
            ApplicationSessionAdmissionError::Capacity(limit) => map_capacity_error(limit),
            ApplicationSessionAdmissionError::ReplayConflict => {
                Error::from_registered(codes::STATE_CONFLICT)
            }
            ApplicationSessionAdmissionError::ScopeNotGranted { .. } => {
                Error::from_registered(codes::AUTHORITY_CONFLICT)
            }
        },
        ApplicationSessionWorkflowError::ModelInvalid(_) => {
            Error::from_registered(codes::REQUEST_INVALID)
        }
        ApplicationSessionWorkflowError::State(error) => map_state_error(error),
    }
}

const fn map_capacity_error(_limit: ApplicationCapacityLimit) -> Error {
    Error::from_registered(codes::CAPACITY_LIMIT)
}

fn map_state_error(error: ApplicationSessionStateError) -> Error {
    match error {
        ApplicationSessionStateError::ActiveGlobalCapacity
        | ApplicationSessionStateError::ActiveSubjectCapacity
        | ApplicationSessionStateError::ReplayGlobalCapacity
        | ApplicationSessionStateError::ReplaySubjectCapacity
        | ApplicationSessionStateError::SessionRecordTooLarge
        | ApplicationSessionStateError::StableStateTooLarge
        | ApplicationSessionStateError::IndexStateTooLarge => {
            Error::from_registered(codes::CAPACITY_LIMIT)
        }
        ApplicationSessionStateError::AuthorityGenerationMismatch
        | ApplicationSessionStateError::ReplayAlreadyExists => {
            Error::from_registered(codes::STATE_CONFLICT)
        }
        ApplicationSessionStateError::AuthorityGenerationExhausted => {
            Error::from_registered(codes::VERSION_CAPACITY)
        }
        ApplicationSessionStateError::IndexesUnavailable
        | ApplicationSessionStateError::DuplicateCaller
        | ApplicationSessionStateError::DuplicateProofFingerprint
        | ApplicationSessionStateError::InvalidAuthorityBinding
        | ApplicationSessionStateError::InvalidSessionRecord
        | ApplicationSessionStateError::InvalidReplayRecord
        | ApplicationSessionStateError::SessionReplayMismatch
        | ApplicationSessionStateError::FutureAuthorityGeneration
        | ApplicationSessionStateError::EncodingFailed => Error::from(InternalError::unavailable()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        ids::{CanisterRole, FleetKey},
        model::auth::application_authorization::ApplicationScope,
        test::support::fleet_key,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn session(
        subject: Principal,
        fleet: FleetKey,
        role: CanisterRole,
        generation: u64,
        expires_at_ns: u64,
    ) -> LocalApplicationSession {
        LocalApplicationSession::new(
            subject,
            subject,
            p(9),
            fleet,
            role,
            CanonicalApplicationScopes::for_session(vec![
                ApplicationScope::parse("app:read").unwrap(),
            ])
            .unwrap(),
            generation,
            10,
            expires_at_ns,
            [1; 32],
            [2; 32],
        )
        .unwrap()
    }

    #[test]
    fn inactive_status_uses_the_frozen_denial_precedence() {
        let fleet = fleet_key(1);
        let role = CanisterRole::new("component");
        let authority = LocalApplicationAuthoritySnapshot::new(fleet, role.clone(), 7);

        assert_eq!(
            classify_inactive_session(
                &session(
                    Principal::anonymous(),
                    fleet_key(2),
                    CanisterRole::new("other"),
                    6,
                    20,
                ),
                &authority,
                20,
            ),
            Some(InactiveApplicationSession::Expired { expired_at_ns: 20 })
        );
        assert_eq!(
            classify_inactive_session(
                &session(p(1), fleet_key(2), CanisterRole::new("other"), 6, 100),
                &authority,
                20,
            ),
            Some(InactiveApplicationSession::StaleFleet)
        );
        assert_eq!(
            classify_inactive_session(
                &session(p(1), fleet, CanisterRole::new("other"), 6, 100),
                &authority,
                20,
            ),
            Some(InactiveApplicationSession::StaleRole)
        );
        assert_eq!(
            classify_inactive_session(&session(p(1), fleet, role.clone(), 6, 100), &authority, 20),
            Some(InactiveApplicationSession::StaleGeneration {
                session_generation: 6,
                current_generation: 7,
            })
        );
        assert_eq!(
            classify_inactive_session(
                &session(Principal::anonymous(), fleet, role.clone(), 7, 100),
                &authority,
                20,
            ),
            Some(InactiveApplicationSession::InadmissibleSubject)
        );
        assert_eq!(
            classify_inactive_session(&session(p(1), fleet, role, 7, 100), &authority, 20),
            None
        );
    }

    #[test]
    fn configured_scope_and_ttl_policy_are_enforced_at_the_adapter_boundary() {
        let config = LocalApplicationAuthorizationConfig {
            allowed_scopes: vec!["app:read".to_string(), "app:write".to_string()],
            default_session_ttl_secs: 10,
            maximum_session_ttl_secs: 20,
        };
        let read = CanonicalApplicationScopes::for_session(vec![
            ApplicationScope::parse("app:read").unwrap(),
        ])
        .unwrap();
        let admin = CanonicalApplicationScopes::for_session(vec![
            ApplicationScope::parse("app:admin").unwrap(),
        ])
        .unwrap();

        assert!(require_configured_scopes(&config, &read).is_ok());
        assert!(require_configured_scopes(&config, &admin).is_err());
        assert_eq!(session_expiry(100, None, &config).unwrap(), 10_000_000_100);
        assert_eq!(
            session_expiry(100, Some(20), &config).unwrap(),
            20_000_000_100
        );
        assert!(session_expiry(100, Some(0), &config).is_err());
        assert!(session_expiry(100, Some(21), &config).is_err());
    }
}
