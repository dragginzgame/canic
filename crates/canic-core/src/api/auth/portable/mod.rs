//! Module: api::auth::portable
//!
//! Responsibility: acquire runtime caller/time and project receiver results into Auth Wire.
//! Boundary: no request field selects trusted authority, target caller or enrollment.

use crate::{
    access::auth::{AuthorizedApplicationSubject, LocalApplicationAuthorizationDecision},
    ops::ic::IcOps,
    workflow::runtime::auth::portable::PortableSessionWorkflow,
};

pub use ic_auth_wire::model::{AdmissionRefusal, LogoutResult, SessionReadRefusal};
pub use ic_auth_wire::{EstablishSession, Logout, ReadSession, SessionInput, SessionView};

/// Establish a session for the actual ingress browser caller.
pub fn establish(input: SessionInput) -> Result<SessionView, AdmissionRefusal> {
    PortableSessionWorkflow::establish(IcOps::msg_caller().into(), IcOps::now_nanos(), input)
        .map_err(|error| {
            crate::log!(
                crate::log::Topic::Auth,
                Warn,
                "portable session admission refused: {error}"
            );
            error.admission_refusal()
        })
}

/// Read only the actual caller's session after complete current authority checks.
pub fn read() -> Result<SessionView, SessionReadRefusal> {
    PortableSessionWorkflow::read(
        IcOps::msg_caller().into(),
        IcOps::now_nanos(),
        Vec::new(),
        IcOps::in_replicated_execution(),
    )
    .map_err(|error| error.read_refusal())
}

/// Commit this caller's local logout even after proof enrollment becomes unavailable.
#[must_use]
pub fn logout() -> LogoutResult {
    PortableSessionWorkflow::logout(IcOps::msg_caller().into(), IcOps::now_nanos())
        .unwrap_or_else(|error| error.logout_unavailable())
        .into()
}

pub(crate) fn authorize(
    caller: crate::cdk::types::Principal,
    scope: &str,
) -> LocalApplicationAuthorizationDecision {
    match PortableSessionWorkflow::read(
        caller.into(),
        IcOps::now_nanos(),
        vec![scope.to_owned()],
        IcOps::in_replicated_execution(),
    ) {
        Ok(session) => LocalApplicationAuthorizationDecision::Allow(AuthorizedApplicationSubject {
            issuer: session.issuer.into(),
            subject: session.subject.into(),
            expires_at_ns: session.expires_at_ns,
        }),
        Err(error) => LocalApplicationAuthorizationDecision::Deny(error.denial()),
    }
}

/// Observe retained session metadata after the mounted endpoint authenticates Root.
pub fn inspect(
    request: canic_contracts::dto::portable_inspection::Request,
) -> Result<
    canic_contracts::dto::portable_inspection::Page,
    canic_contracts::dto::portable_inspection::Refusal,
> {
    PortableSessionWorkflow::inspect(request, IcOps::now_nanos())
}

/// Inspect declared and currently effective policy after Root authentication.
pub fn inspect_policy() -> Result<
    canic_contracts::dto::portable_inspection::PolicyReport,
    canic_contracts::dto::portable_inspection::Refusal,
> {
    PortableSessionWorkflow::inspect_policy(IcOps::now_nanos())
}
