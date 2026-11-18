//! Install, refresh and authorize the volatile receiver on the IC.
use super::{Policy, STATE, SessionReadRefusal, SessionView, enrollment, initialize, read_scope};
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
    init(
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
    );
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
            .prepare(&mut s.receiver, challenge, api::time())
    })?;
    let response = pull.fetch().await?;
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().ok_or(enrollment::Error::Closed)?;
        s.enrollment.finish(&mut s.receiver, response, api::time())
    })
}

/// Authorize the actual caller for this operation using the current time.
/// After an await, use [`reauthorize`] with the originally admitted session.
/// # Errors
/// Returns the current enrollment or session refusal for this scope.
pub fn authorize(scope: &str) -> Result<SessionView, SessionReadRefusal> {
    read_scope(api::caller(), api::time(), scope)
}

/// Check the original browser and account again before a subsequent effect.
/// # Errors
/// Refuses stale sessions, a changed account or insufficient current scope.
pub fn reauthorize(original: &SessionView, scope: &str) -> Result<(), SessionReadRefusal> {
    let current = read_scope(original.caller, api::time(), scope)?;
    if current.issuer == original.issuer && current.subject == original.subject {
        Ok(())
    } else {
        Err(SessionReadRefusal::AuthorityUnavailable)
    }
}
