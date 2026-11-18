//! One volatile Ed25519 receiver for a canister accepting a single issuer and role.
//!
//! Every verified subject in the configured grant can request a session. Applications
//! select required scopes and authorize resources in their own handlers. Use the
//! lower-level receiver interfaces for other admission policies.
//!
//! Mount the existing Auth endpoints with `admit`, an application-selected scope
//! reader and `logout`. Mount `refresh` through the controller-guarded refresh
//! endpoint. Call `init` only from the IC install hook. Recreating the receiver
//! after an upgrade would discard its replay and logout history.
use super::{
    enrollment::{
        self, supplier as ic_auth,
        types::{AuthRole, PortableMechanism, Principal},
    },
    protocol::{
        SessionInput, SessionView,
        model::{AdmissionRefusal, LogoutResult, LogoutUnavailable, SessionReadRefusal},
    },
};
use ic_auth::{
    receiver_memory::{
        AdmissionHost, LogoutError, LogoutReceipt, MemoryConfig, MemoryError, MemoryReceiver,
        SessionAuthority, SessionPolicyError, SessionReceipt,
    },
    session_request::{SessionRequest, SessionTtlPolicy},
    verification::{Ed25519Key, VerificationContext, VerificationProfile},
};
#[cfg(target_arch = "wasm32")]
use ic_platform_runtime::system as api;
use std::cell::RefCell;

struct State {
    enrollment: enrollment::Receiver,
    receiver: MemoryReceiver,
    policy: Policy,
    max_login_ttl_ns: u64,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

/// Installed acceptance policy for credentials from one issuer and role.
pub struct Policy {
    /// Accepted issuer from the protected install configuration.
    pub issuer: Principal,
    /// Required role in the current Root grant.
    pub role: AuthRole,
    /// Default and maximum admitted session lifetime.
    pub session_ttl: SessionTtlPolicy,
    /// Upper bound on the credential proof lifetime.
    pub max_proof_ttl_ns: u64,
}

/// Initialize only during fresh installation. Upgrade must leave volatile state closed.
/// # Panics
/// Traps on invalid configuration or if an active receiver already exists.
#[cfg(target_arch = "wasm32")]
pub fn init(memory: MemoryConfig, policy: Policy) {
    initialize(memory, policy, api::canister_self(), api::time());
}

pub(super) fn initialize(memory: MemoryConfig, policy: Policy, own_id: Principal, now: u64) {
    assert_eq!(memory.enrollment.receiver, own_id);
    assert_ne!(
        memory.enrollment.authority, own_id,
        "receiver cannot enroll itself"
    );
    assert!(
        policy.max_proof_ttl_ns > 0
            && policy.issuer != Principal::anonymous()
            && policy.issuer != Principal::management_canister(),
        "invalid issuer or proof TTL"
    );
    let max_login_ttl_ns = memory.login.max_login_ttl_ns;
    let enrollment = enrollment::Receiver::new(memory.enrollment, memory.audience, now)
        .expect("installed enrollment binding");
    let receiver = MemoryReceiver::new_fresh(memory, now).expect("fresh session receiver");
    STATE.with_borrow_mut(|slot| {
        assert!(slot.is_none(), "receiver already initialized");
        *slot = Some(State {
            enrollment,
            receiver,
            policy,
            max_login_ttl_ns,
        });
    });
}
/// Open Auth's bounded starting profile from the approved fresh-install envelope.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub fn install(input: &super::install::Install) {
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
#[cfg(target_arch = "wasm32")]
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

fn view(receipt: SessionReceipt) -> SessionView {
    SessionView {
        caller: receipt.caller,
        issuer: receipt.issuer,
        subject: receipt.subject,
        expires_at_ns: receipt.expires_at_ns,
    }
}
fn admission_error(error: MemoryError) -> AdmissionRefusal {
    match error {
        MemoryError::Closed | MemoryError::Enrollment(_) => AdmissionRefusal::Unavailable,
        MemoryError::Policy(SessionPolicyError::Revoked) => AdmissionRefusal::Revoked,
        MemoryError::Policy(SessionPolicyError::ReplayConflict) => AdmissionRefusal::ReplayConflict,
        MemoryError::Request(_) | MemoryError::Policy(SessionPolicyError::ScopeRejected) => {
            AdmissionRefusal::ScopeRejected
        }
        MemoryError::Policy(SessionPolicyError::Capacity | SessionPolicyError::ByteCapacity) => {
            AdmissionRefusal::Capacity
        }
        MemoryError::Policy(_) => AdmissionRefusal::AuthorityUnavailable,
        MemoryError::Proof(ic_auth::verification::CredentialError::Ed25519(
            ic_auth::verification::VerificationError::Budget(_),
        ))
        | MemoryError::Budget(_) => AdmissionRefusal::Throttled,
        MemoryError::Claims(_) | MemoryError::Context(_) | MemoryError::Proof(_) => {
            AdmissionRefusal::InvalidCredential
        }
    }
}
/// Admit a browser-bound session against the current enrollment grant.
///
/// # Panics
/// Panics if an admitted grant violates its validated key, role or grant invariants.
pub fn admit(
    caller: Principal,
    input: SessionInput,
    now: u64,
) -> Result<SessionView, AdmissionRefusal> {
    let credential = input
        .credential
        .try_into()
        .map_err(|_| AdmissionRefusal::InvalidCredential)?;
    let request = SessionRequest::new(
        input
            .scopes
            .into_vec()
            .into_iter()
            .map(|scope| scope.into_string())
            .collect(),
        input.ttl_secs,
    )
    .map_err(|_| AdmissionRefusal::ScopeRejected)?;
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(AdmissionRefusal::Unavailable)?;

        let snapshot = s
            .enrollment
            .current(now)
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        if snapshot.grant.issuer != s.policy.issuer
            || snapshot.grant.grant.target.as_str() != s.policy.role.as_str()
        {
            return Err(AdmissionRefusal::Unavailable);
        }
        if snapshot.revoked {
            return Err(AdmissionRefusal::Revoked);
        }
        let audience = snapshot.grant.audience();
        let grants = snapshot.grant.grants().unwrap();

        let key = Ed25519Key::new(snapshot.grant.key).unwrap();
        let context = VerificationContext {
            audience: &audience,
            grants: &grants,
            issuer: snapshot.grant.issuer,
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: key.binding(),
            caller,
            now_ns: now,
            authority_not_before_ns: snapshot.grant.not_before_ns,
            authority_expires_at_ns: snapshot.grant.expires_at_ns,
            enrollment_expires_at_ns: snapshot.expires_at_ns,
            min_login_started_at_ns: 0,
            min_proof_issued_at_ns: 0,
            max_login_ttl_ns: s.max_login_ttl_ns,
            max_proof_ttl_ns: s.policy.max_proof_ttl_ns,
        };
        s.receiver.prune(now, 8).map_err(admission_error)?;
        let outcome = s
            .receiver
            .admit(
                &credential,
                request,
                AdmissionHost {
                    verifier: VerificationProfile::Ed25519 {
                        context: &context,
                        key: &key,
                    },
                    role: &s.policy.role,
                    ttl: s.policy.session_ttl,
                    generation: snapshot.generation,
                    enabled: !snapshot.revoked,
                    subject_admissible: true,
                },
            )
            .map_err(admission_error)?;

        Ok(view(outcome.receipt))
    })
}
/// Read this caller's session with the application-selected scope.
/// # Panics
/// Panics if an installed grant violates validated key or grants invariants.
pub fn read_scope(
    caller: Principal,
    now: u64,
    scope: &str,
) -> Result<SessionView, SessionReadRefusal> {
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(SessionReadRefusal::Unavailable)?;
        let snapshot = s
            .enrollment
            .current(now)
            .map_err(|_| SessionReadRefusal::Unavailable)?;
        if snapshot.grant.issuer != s.policy.issuer
            || snapshot.grant.grant.target.as_str() != s.policy.role.as_str()
        {
            return Err(SessionReadRefusal::Unavailable);
        }
        if snapshot.revoked {
            return Err(SessionReadRefusal::Revoked);
        }
        let grants = snapshot.grant.grants().unwrap();

        let authority = SessionAuthority {
            caller,
            now_ns: now,
            audience: snapshot.grant.audience(),
            issuer: snapshot.grant.issuer,
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: Ed25519Key::new(snapshot.grant.key).unwrap().binding(),
            role: &s.policy.role,
            grants: grants.as_slice(),
            generation: snapshot.generation,
            not_before_ns: snapshot.grant.not_before_ns,
            expires_at_ns: snapshot.grant.expires_at_ns,
            enrollment_expires_at_ns: snapshot.expires_at_ns,
            min_login_started_at_ns: 0,
            max_session_ttl_secs: s.policy.session_ttl.maximum_secs(),
            enabled: !snapshot.revoked,
            subject_admissible: true,
        };
        s.receiver
            .read(
                &authority,
                &[scope
                    .parse()
                    .map_err(|_| SessionReadRefusal::ScopeRejected)?],
            )
            .map(view)
            .map_err(|error| match error {
                MemoryError::Closed | MemoryError::Enrollment(_) => SessionReadRefusal::Unavailable,
                MemoryError::Policy(SessionPolicyError::MissingOrExpired) => {
                    SessionReadRefusal::MissingOrExpired
                }
                MemoryError::Policy(SessionPolicyError::Revoked) => SessionReadRefusal::Revoked,
                MemoryError::Policy(SessionPolicyError::ScopeRejected) => {
                    SessionReadRefusal::ScopeRejected
                }
                _ => SessionReadRefusal::AuthorityUnavailable,
            })
    })
}
/// Revoke this caller’s receiver session and return its replay denial deadline.
#[must_use]
pub fn logout(caller: Principal, now: u64) -> LogoutResult {
    STATE.with_borrow_mut(|state| {
        let Some(s) = state else {
            return LogoutResult::Unavailable(LogoutUnavailable::Storage);
        };

        match s.receiver.logout(caller, now) {
            Ok(LogoutReceipt::Revoked { deny_until_ns }) => LogoutResult::Revoked(deny_until_ns),
            Ok(LogoutReceipt::Unconfirmed { deny_until_ns }) => {
                LogoutResult::Unconfirmed(deny_until_ns)
            }
            Err(LogoutError::InvalidCaller) => LogoutResult::InvalidCaller,
            Err(LogoutError::Capacity) => LogoutResult::Unavailable(LogoutUnavailable::Capacity),
            Err(LogoutError::Clock) => LogoutResult::Unavailable(LogoutUnavailable::Clock),
            Err(LogoutError::Closed | LogoutError::InvalidFacts) => {
                LogoutResult::Unavailable(LogoutUnavailable::Storage)
            }
        }
    })
}

/// Authorize the actual caller for this operation using the current time.
/// After an await, use [`reauthorize`] with the originally admitted session.
/// # Errors
/// Returns the current enrollment or session refusal for this scope.
#[cfg(target_arch = "wasm32")]
pub fn authorize(scope: &str) -> Result<SessionView, SessionReadRefusal> {
    read_scope(api::caller(), api::time(), scope)
}

/// Check the original browser and account again before a subsequent effect.
/// # Errors
/// Refuses stale sessions, a changed account or insufficient current scope.
#[cfg(target_arch = "wasm32")]
pub fn reauthorize(original: &SessionView, scope: &str) -> Result<(), SessionReadRefusal> {
    let current = read_scope(original.caller, api::time(), scope)?;
    if current.issuer == original.issuer && current.subject == original.subject {
        Ok(())
    } else {
        Err(SessionReadRefusal::AuthorityUnavailable)
    }
}

/// Mount the standard volatile receiver and its controller-only refresh.
/// The application selects the read endpoint's scope and explicitly initializes on install.
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! mount_auth_session {
    (read = $scope:literal $(,)?) => {
        mod canic_session {
            use $crate::auth::{
                enrollment::types::Principal,
                protocol::{SessionView, model::SessionReadRefusal},
                session,
            };
            fn read(caller: Principal, now: u64) -> Result<SessionView, SessionReadRefusal> {
                session::read_scope(caller, now, $scope)
            }
            $crate::mount_auth_endpoints! {
                admit = $crate::auth::session::admit,
                read = super::read,
                logout = $crate::auth::session::logout,
            }
            $crate::mount_auth_refresh_endpoint!($crate::auth::session::refresh);
        }
    };
}

#[cfg(test)]
mod tests;

/// Install a configured receiver and its app in one hook. An upgrade reopens only
/// the application's retained state. Both hooks trap on decoding or app failure.
/// The app decoder receives its own bytes. The app opens its store with
/// `Some(decoded)` on install and `None` on upgrade.
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! mount_auth_lifecycle {
    (decode = $decode:path, open = $open:path $(,)?) => {
        mod canic_auth_lifecycle {
            use $crate::auth::{endpoint::{runtime, fail}, install::Install, session};
            fn allow() -> Result<(), std::convert::Infallible> { Ok(()) }
            fn empty(bytes: &[u8]) -> Result<(), &'static str> {
                bytes.is_empty().then_some(()).ok_or("expected empty upgrade arguments")
            }
            #[runtime::init(runtime = runtime, guard = allow, decode = Install::decode, max_bytes = 4096, on_error = fail)]
            #[allow(clippy::needless_pass_by_value)]
            fn init(input: Install) {
                let application = $decode(&input.application).unwrap_or_else(|error| fail(error));
                session::install(&input);
                $open(Some(application));
            }
            #[runtime::post_upgrade(runtime = runtime, guard = allow, decode = empty, max_bytes = 0, on_error = fail)]
            fn post_upgrade((): ()) { $open(None); }
        }
    };
}
