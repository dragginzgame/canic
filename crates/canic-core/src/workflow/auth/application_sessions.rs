//! Module: workflow::auth::application_sessions
//!
//! Responsibility: run IC Auth session admission against Canic's durable transaction.
//! Boundary: Canic supplies protected identity and policy; IC Auth owns replay and leases.

#[cfg(feature = "auth-local-application-authorization")]
use crate::ops::{auth::AuthOps, storage::auth::application_sessions::ApplicationSessionStore};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::DelegatedToken,
    model::auth::application_authorization::{CanonicalApplicationScopes, LocalApplicationSession},
    ops::auth::application_authorization::LocalApplicationAuthorizationAuthority,
};

/// Authenticated caller and protected inputs for one synchronous admission.
#[cfg_attr(
    not(feature = "auth-local-application-authorization"),
    expect(
        dead_code,
        reason = "session input is rejected when the capability is absent"
    )
)]
pub struct ApplicationSessionEstablishInput {
    pub token: DelegatedToken,
    pub caller: Principal,
    pub requested_scopes: CanonicalApplicationScopes,
    pub requested_ttl_secs: Option<u64>,
    pub authority: LocalApplicationAuthorizationAuthority,
    pub now_ns: u64,
}

/// Observable result selected by the shared session engine.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(feature = "auth-local-application-authorization"),
    expect(
        dead_code,
        reason = "session outcomes are produced only by the enabled capability"
    )
)]
pub enum ApplicationSessionEstablishResult {
    Created(LocalApplicationSession),
    ExactRetry(LocalApplicationSession),
    Replaced(LocalApplicationSession),
}

/// Stateless coordinator for the current session contract.
pub struct ApplicationSessionWorkflow;

impl ApplicationSessionWorkflow {
    /// Verify/admit or exactly replay one request without extending a retained lease.
    pub fn establish(
        input: ApplicationSessionEstablishInput,
    ) -> Result<ApplicationSessionEstablishResult, InternalError> {
        #[cfg(feature = "auth-local-application-authorization")]
        {
            let verifier = AuthOps::resolve_token_authority(&input.authority)?;
            let token = AuthOps::application_session_token(&input.token)?;
            let request = AuthOps::shared_application_session_request(
                &input.requested_scopes,
                input.requested_ttl_secs,
            )?;
            let policy = AuthOps::shared_application_session_policy(&input.authority, true);
            let ttl = AuthOps::delegated_token_max_ttl_ns()?;
            let context = verifier.context(input.caller, &[], input.now_ns, ttl, ttl);
            let mut store = ApplicationSessionStore::new(&input.authority.snapshot);
            let established =
                ic_auth::session::establish_session(&mut store, &token, &request, &context, policy)
                    .map_err(AuthOps::map_application_session_error)?;
            let session = ApplicationSessionStore::project_committed(input.caller)?;
            Ok(match established {
                ic_auth::session::SessionEstablished::Created(_) => {
                    ApplicationSessionEstablishResult::Created(session)
                }
                ic_auth::session::SessionEstablished::ExactRetry(_) => {
                    ApplicationSessionEstablishResult::ExactRetry(session)
                }
                ic_auth::session::SessionEstablished::Replaced(_) => {
                    ApplicationSessionEstablishResult::Replaced(session)
                }
            })
        }
        #[cfg(not(feature = "auth-local-application-authorization"))]
        {
            let _ = input;
            Err(InternalError::public(
                crate::diagnostics::codes::CONFIGURATION_INACTIVE,
            ))
        }
    }

    /// Recheck retained session authority against the current protected Root policy.
    #[cfg(feature = "auth-local-application-authorization")]
    pub fn authorize(
        caller: Principal,
        required_scope: Option<&str>,
        now_ns: u64,
        authority: &LocalApplicationAuthorizationAuthority,
        subject_admissible: bool,
    ) -> Result<ic_auth::session::Session, ic_auth::session::SessionError> {
        let verifier = AuthOps::resolve_token_authority(authority)
            .map_err(|_| ic_auth::session::SessionError::StaleAuthority)?;
        let required = required_scope
            .map(str::to_owned)
            .into_iter()
            .collect::<Vec<_>>();
        let ttl = AuthOps::delegated_token_max_ttl_ns()
            .map_err(|_| ic_auth::session::SessionError::StaleAuthority)?;
        let context = verifier.context(caller, &required, now_ns, ttl, ttl);
        let policy = AuthOps::shared_application_session_policy(authority, subject_admissible);
        let mut store = ApplicationSessionStore::new(&authority.snapshot);
        ic_auth::session::authorize_session(&mut store, &context, policy)
    }
}
