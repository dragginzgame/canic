//! Module: workflow::auth::application_sessions
//!
//! Responsibility: sequence exact retry, proof eligibility, scope narrowing and atomic session commit.
//! Does not own: proof decoding/verification, caller/time reads, protected TTL policy, or endpoint DTOs.
//! Boundary: endpoint workflow supplies one verified authority projection and canonical request identity.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "B3 workflow is consumed by the sequenced B4 endpoint variants"
    )
)]

use crate::{
    domain::policy::pure::auth::application_authorization::{
        ApplicationProofEligibilityError, ApplicationReplayDisposition,
        ApplicationSessionAdmissionDecision, ApplicationSessionAdmissionError,
        ApplicationSessionAdmissionInput, decide_application_session_admission,
        narrow_application_session_scopes, validate_application_proof_eligibility,
    },
    model::auth::application_authorization::{
        ApplicationAuthorityModelError, CanonicalApplicationScopes, LocalApplicationReplay,
        LocalApplicationSession, VerifiedApplicationAuthority,
    },
    ops::storage::auth::{
        AuthStateOps,
        application_sessions::{
            ApplicationReplayResolution, ApplicationSessionCommitResult,
            ApplicationSessionStateError,
        },
    },
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const APPLICATION_SESSION_REQUEST_HASH_DOMAIN: &[u8] = b"canic-application-session-request-v1";

/// Fully verified, target-local inputs required for one canonical session commit.
pub struct ApplicationSessionEstablishInput {
    pub authority: VerifiedApplicationAuthority,
    pub requested_scopes: CanonicalApplicationScopes,
    pub authority_generation: u64,
    pub established_at_ns: u64,
    pub expires_at_ns: u64,
    pub establishment_request_hash: [u8; 32],
}

/// Created, replaced or byte-identical current session selected by establishment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationSessionEstablishResult {
    Created(LocalApplicationSession),
    ExactRetry(LocalApplicationSession),
    Replaced(LocalApplicationSession),
}

/// Closed state-workflow failure; endpoint mapping remains owned by B4.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ApplicationSessionWorkflowError {
    #[error("application proof is not eligible for local session establishment")]
    ProofIneligible(ApplicationProofEligibilityError),

    #[error("application session admission was denied")]
    AdmissionDenied(ApplicationSessionAdmissionError),

    #[error("application session model construction failed")]
    ModelInvalid(ApplicationAuthorityModelError),

    #[error(transparent)]
    State(#[from] ApplicationSessionStateError),
}

/// Stateless coordinator for current-format local application session state.
pub struct ApplicationSessionWorkflow;

impl ApplicationSessionWorkflow {
    /// Hash one canonical narrowing request without treating token bytes as request authority.
    #[must_use]
    pub fn establishment_request_hash(
        requested_scopes: &CanonicalApplicationScopes,
        requested_ttl_secs: Option<u64>,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(APPLICATION_SESSION_REQUEST_HASH_DOMAIN);
        hasher.update((requested_scopes.as_slice().len() as u64).to_be_bytes());
        for scope in requested_scopes.as_slice() {
            hasher.update((scope.as_str().len() as u64).to_be_bytes());
            hasher.update(scope.as_str().as_bytes());
        }
        match requested_ttl_secs {
            None => hasher.update([0]),
            Some(ttl_secs) => {
                hasher.update([1]);
                hasher.update(ttl_secs.to_be_bytes());
            }
        }
        hasher.finalize().into()
    }

    /// Resolve exact retry or conflicting proof reuse before proof verification.
    pub fn resolve_retry(
        proof_fingerprint: [u8; 32],
        caller: crate::cdk::types::Principal,
        establishment_request_hash: [u8; 32],
        now_ns: u64,
    ) -> Result<ApplicationReplayResolution, ApplicationSessionStateError> {
        AuthStateOps::resolve_application_replay(
            proof_fingerprint,
            caller,
            caller,
            establishment_request_hash,
            now_ns,
        )
    }

    /// Commit one already-verified proof as a canonical local application session.
    pub fn establish_verified(
        input: ApplicationSessionEstablishInput,
    ) -> Result<ApplicationSessionEstablishResult, ApplicationSessionWorkflowError> {
        if input.authority_generation != AuthStateOps::application_authority_generation() {
            return Err(ApplicationSessionWorkflowError::State(
                ApplicationSessionStateError::AuthorityGenerationMismatch,
            ));
        }
        let caller = input.authority.presenter();
        let proof_fingerprint = input.authority.proof_fingerprint();
        match Self::resolve_retry(
            proof_fingerprint,
            caller,
            input.establishment_request_hash,
            input.established_at_ns,
        )? {
            ApplicationReplayResolution::ExactActive(session) => {
                return Ok(ApplicationSessionEstablishResult::ExactRetry(*session));
            }
            ApplicationReplayResolution::Conflict => {
                return Err(ApplicationSessionWorkflowError::AdmissionDenied(
                    ApplicationSessionAdmissionError::ReplayConflict,
                ));
            }
            ApplicationReplayResolution::Absent => {}
        }

        validate_application_proof_eligibility(&input.authority, caller, input.established_at_ns)
            .map_err(ApplicationSessionWorkflowError::ProofIneligible)?;
        let scopes =
            narrow_application_session_scopes(input.authority.scopes(), input.requested_scopes)
                .map_err(ApplicationSessionWorkflowError::AdmissionDenied)?;

        let current_session = AuthStateOps::application_session(caller)?;
        let capacity = AuthStateOps::application_session_capacity(caller)?;
        let admission = decide_application_session_admission(ApplicationSessionAdmissionInput {
            replay: ApplicationReplayDisposition::Absent,
            replacing_existing_session: current_session.is_some(),
            capacity,
        })
        .map_err(ApplicationSessionWorkflowError::AdmissionDenied)?;

        let session = LocalApplicationSession::new(
            caller,
            input.authority.subject(),
            input.authority.issuer(),
            input.authority.fleet(),
            input.authority.role().clone(),
            scopes,
            input.authority_generation,
            input.established_at_ns,
            input.expires_at_ns,
            proof_fingerprint,
            input.establishment_request_hash,
        )
        .map_err(ApplicationSessionWorkflowError::ModelInvalid)?;
        let replay = LocalApplicationReplay::new(
            proof_fingerprint,
            caller,
            input.authority.subject(),
            input.authority_generation,
            input.authority.proof_expires_at_ns(),
        )
        .map_err(ApplicationSessionWorkflowError::ModelInvalid)?;
        let committed = AuthStateOps::commit_application_session(session.clone(), replay)?;
        match (admission, committed) {
            (
                ApplicationSessionAdmissionDecision::CommitNew,
                ApplicationSessionCommitResult::Created,
            ) => Ok(ApplicationSessionEstablishResult::Created(session)),
            (
                ApplicationSessionAdmissionDecision::CommitReplacement,
                ApplicationSessionCommitResult::Replaced,
            ) => Ok(ApplicationSessionEstablishResult::Replaced(session)),
            _ => Err(ApplicationSessionWorkflowError::State(
                ApplicationSessionStateError::SessionReplayMismatch,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        ids::CanisterRole,
        model::auth::application_authorization::{ApplicationScope, CanonicalApplicationScopes},
        storage::stable::auth::{AuthState, AuthStateData},
        test::{seams, support::fleet_key},
    };

    struct StateGuard(AuthStateData);

    impl StateGuard {
        fn empty() -> Self {
            let original = AuthState::export();
            AuthState::import(AuthStateData::default());
            AuthStateOps::restore_application_session_state().unwrap();
            Self(original)
        }
    }

    impl Drop for StateGuard {
        fn drop(&mut self) {
            AuthState::import(self.0.clone());
            AuthStateOps::restore_application_session_state().unwrap();
        }
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn scopes(values: &[&str], verified: bool) -> CanonicalApplicationScopes {
        let scopes = values
            .iter()
            .map(|value| ApplicationScope::parse(*value).unwrap())
            .collect();
        if verified {
            CanonicalApplicationScopes::for_verified_grant(scopes).unwrap()
        } else {
            CanonicalApplicationScopes::for_session(scopes).unwrap()
        }
    }

    fn authority() -> VerifiedApplicationAuthority {
        VerifiedApplicationAuthority::new(
            p(1),
            p(1),
            p(2),
            fleet_key(3),
            CanisterRole::new("component"),
            scopes(&["app:read", "app:write"], true),
            10,
            10,
            70,
            [4; 32],
        )
        .unwrap()
    }

    fn input(now_ns: u64, request_hash: u8) -> ApplicationSessionEstablishInput {
        ApplicationSessionEstablishInput {
            authority: authority(),
            requested_scopes: scopes(&["app:read"], false),
            authority_generation: 0,
            established_at_ns: now_ns,
            expires_at_ns: 1_000,
            establishment_request_hash: [request_hash; 32],
        }
    }

    #[test]
    fn exact_retry_returns_the_committed_session_after_proof_expiry_without_extension() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let created = ApplicationSessionWorkflow::establish_verified(input(20, 5)).unwrap();
        let ApplicationSessionEstablishResult::Created(created) = created else {
            panic!("first establishment must create");
        };

        let retried = ApplicationSessionWorkflow::establish_verified(input(80, 5)).unwrap();
        assert_eq!(
            retried,
            ApplicationSessionEstablishResult::ExactRetry(created)
        );
    }

    #[test]
    fn conflicting_request_hash_is_denied_before_expired_proof_eligibility() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        ApplicationSessionWorkflow::establish_verified(input(20, 5)).unwrap();
        assert_eq!(
            ApplicationSessionWorkflow::establish_verified(input(80, 6)),
            Err(ApplicationSessionWorkflowError::AdmissionDenied(
                ApplicationSessionAdmissionError::ReplayConflict,
            ))
        );
    }

    #[test]
    fn establishment_request_hash_binds_the_canonical_scope_set_and_ttl_choice() {
        let read = scopes(&["app:read"], false);
        let read_write = scopes(&["app:write", "app:read"], false);

        assert_eq!(
            ApplicationSessionWorkflow::establishment_request_hash(&read_write, Some(30)),
            ApplicationSessionWorkflow::establishment_request_hash(
                &scopes(&["app:read", "app:write"], false),
                Some(30),
            )
        );
        assert_ne!(
            ApplicationSessionWorkflow::establishment_request_hash(&read, None),
            ApplicationSessionWorkflow::establishment_request_hash(&read, Some(30))
        );
        assert_ne!(
            ApplicationSessionWorkflow::establishment_request_hash(&read, Some(30)),
            ApplicationSessionWorkflow::establishment_request_hash(&read_write, Some(30))
        );
    }
}
