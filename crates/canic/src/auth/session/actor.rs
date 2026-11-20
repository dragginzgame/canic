//! Install, refresh and authorize the volatile receiver on the IC.
use super::{Policy, Principal, STATE, SessionReadRefusal, authorized, enrollment, initialize};
use crate::auth::install::Install;
use ic_auth::{receiver_memory::MemoryConfig, session_request::SessionTtlPolicy};
use ic_platform_runtime::system as api;

/// Initialize only during fresh installation. Upgrade must leave volatile state closed.
/// # Panics
/// Traps on invalid configuration or if an active receiver already exists.
pub fn init(memory: MemoryConfig, policy: Policy) {
    initialize(memory, policy, api::canister_self(), api::time());
}

/// Open Auth's bounded starting profile from the approved fresh-install envelope.
#[doc(hidden)]
pub fn install(input: &Install) {
    assert_eq!(
        input.storage,
        crate::auth::install::Storage::Volatile,
        "stable receiver needs governed leases"
    );
    let (memory, policy) = configuration(input);
    init(memory, policy);
}

pub(super) fn configuration(input: &Install) -> (MemoryConfig, Policy) {
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

    /// Check the original browser, issuer, subject, enrollment and scope before another effect.
    /// # Errors
    /// Refuses when the original account no longer has the declared scope.
    pub fn reauthorize(&self) -> Result<(), SessionReadRefusal> {
        if api::caller() != self.caller {
            return Err(SessionReadRefusal::AuthorityUnavailable);
        }
        authorized(self.caller, api::time(), self.scope, |r| {
            if (r.issuer, r.subject) == (self.issuer, self.subject) {
                Ok(())
            } else {
                Err(SessionReadRefusal::AuthorityUnavailable)
            }
        })?
    }
}

/// Invoke the scoped endpoint under the actual caller's current authorization.
/// # Panics
/// Traps if the receiver refuses admission.
#[doc(hidden)]
pub fn with_authorization<R>(scope: &'static str, handler: impl FnOnce(Authorization) -> R) -> R {
    authorized(api::caller(), api::time(), scope, |r| {
        handler(Authorization {
            caller: r.caller,
            issuer: r.issuer,
            subject: r.subject,
            scope,
        })
    })
    .unwrap_or_else(|error| api::trap(&format!("user authorization: {error:?}")))
}
