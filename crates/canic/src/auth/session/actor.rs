//! Install, refresh and authorize the installed receiver on the IC.
use super::{Policy, Principal, STATE, SessionReadRefusal, authorized, enrollment};
use crate::auth::install::Install;
use ic_auth::{receiver_memory::MemoryConfig, session_request::SessionTtlPolicy};
use ic_platform_runtime::system as api;

pub(super) fn configuration(
    input: &Install,
    max_login_secs: Option<u64>,
) -> (MemoryConfig, Policy) {
    use enrollment::types::{AudienceId, CanonicalId};
    use ic_auth::receiver_memory::EnrollmentBinding;
    let binding = EnrollmentBinding {
        authority: input.root,
        receiver: api::canister_self(),
    };
    let audience = AudienceId {
        canonical_network_id: CanonicalId::from_bytes(input.network),
        fleet_id: CanonicalId::from_bytes(input.audience),
    };
    let mut memory = MemoryConfig::starting_profile(audience, binding);
    if let Some(seconds) = max_login_secs {
        memory.login.max_login_ttl_ns = seconds
            .checked_mul(1_000_000_000)
            .expect("login lifetime overflow");
    }
    if let Some(max) = input.limits.max_admitted_logins {
        memory.login.max_admitted_logins = max as usize;
    }
    if let Some(max) = input.limits.max_sessions {
        memory.capacity.max_sessions = max as usize;
    }
    (
        memory,
        Policy {
            issuer: input.issuer,
            role: input.role.parse().expect("installed Auth role"),
            session_ttl: SessionTtlPolicy::new(
                input.limits.default_session_secs,
                input.limits.max_session_secs,
            )
            .expect("installed session TTL"),
            max_proof_ttl_ns: input.limits.max_proof_ttl_ns,
        },
    )
}

/// Refresh the Root grant for the installed receiver after controller admission.
pub async fn refresh() -> Result<(), enrollment::Error> {
    let start = STATE.with_borrow_mut(|s| {
        s.as_mut()
            .ok_or(enrollment::Error::Closed)?
            .enrollment
            .begin(api::time())
    })?;
    let challenge = start.entropy().await?;
    let pull = STATE.with_borrow_mut(|s| {
        let s = s.as_mut().ok_or(enrollment::Error::Closed)?;
        s.enrollment
            .prepare(s.receiver.state(), challenge, api::time())
    })?;
    let response = pull.fetch().await?;
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().ok_or(enrollment::Error::Closed)?;
        s.enrollment
            .finish(s.receiver.state(), response, api::time())
    })
}

/// Identity and required scope captured for one handler invocation.
///
/// An old scoped signature must fail to build even if it imports the session reply
/// under another name. Turning it into Wire input would let a client choose the identity.
///
/// ```compile_fail,E0119
/// use canic::auth::protocol::SessionView as FormerSession;
/// #[canic::query(scope = "read")]
/// fn profile(session: FormerSession) -> bool { session.issuer == session.subject }
/// ```
pub struct Authorization {
    caller: Principal,
    issuer: Principal,
    subject: Principal,
    scope: &'static str,
    user_id: Option<Vec<u8>>,
}

impl Authorization {
    /// Issuer of the admitted account.
    #[must_use]
    pub const fn issuer(&self) -> Principal {
        self.issuer
    }

    /// Stable subject under this issuer.
    #[must_use]
    pub const fn subject(&self) -> Principal {
        self.subject
    }

    /// Optional issuer-attested application identifier. The application owns its encoding.
    #[must_use]
    pub fn user_id(&self) -> Option<&[u8]> {
        self.user_id.as_deref()
    }

    /// Recheck the browser, account, application binding, enrollment and scope before another effect.
    /// # Errors
    /// Refuses when the original account no longer has the declared scope.
    pub fn reauthorize(&self) -> Result<(), SessionReadRefusal> {
        if api::caller() != self.caller {
            return Err(SessionReadRefusal::AuthorityUnavailable);
        }
        authorized(self.caller, api::time(), self.scope, |r| {
            if (r.issuer, r.subject) == (self.issuer, self.subject) && r.ext == self.user_id {
                Ok(())
            } else {
                Err(SessionReadRefusal::AuthorityUnavailable)
            }
        })?
    }
}

/// Admit the current IC caller for an application-selected scope.
/// Call `reauthorize` on the returned authorization after each await and before
/// committing progress or issuing another protected effect.
///
/// # Errors
/// Returns the receiver's typed refusal for missing scope, session or current enrollment.
pub fn authorize(scope: &'static str) -> Result<Authorization, SessionReadRefusal> {
    authorized(api::caller(), api::time(), scope, |r| Authorization {
        caller: r.caller,
        issuer: r.issuer,
        subject: r.subject,
        user_id: r.ext.clone(),
        scope,
    })
}

/// Invoke the scoped endpoint under the actual caller's current authorization.
/// # Panics
/// Traps if the receiver refuses admission.
#[doc(hidden)]
pub fn with_authorization<R>(scope: &'static str, handler: impl FnOnce(Authorization) -> R) -> R {
    let auth = authorize(scope)
        .unwrap_or_else(|error| api::trap(&format!("user authorization: {error:?}")));
    handler(auth)
}
