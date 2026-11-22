//! One Ed25519 receiver for a canister accepting a single issuer and role.
//!
//! Every verified subject in the configured grant can request a session. Applications
//! select required scopes and authorize resources in their own handlers. Use the
//! lower-level receiver interfaces for other admission policies.
//!
//! Mount the existing Auth endpoints with `admit`, an application-selected scope
//! reader and `logout`. Mount `refresh` through the controller-guarded refresh
//! endpoint. A volatile receiver stays closed after upgrade. A stable receiver
//! reopens its checkpoint and requires a fresh Root refresh.

#[cfg(test)]
mod tests;

mod actor;

#[cfg(feature = "auth-receiver-stable")]
pub mod stable;

enum Engine {
    Volatile(MemoryReceiver),
    #[cfg(feature = "auth-receiver-stable")]
    Stable(ic_auth::receiver_memory::StableReceiver),
}

impl Engine {
    fn state(&mut self) -> &mut MemoryReceiver {
        match self {
            Self::Volatile(engine) => engine,
            #[cfg(feature = "auth-receiver-stable")]
            Self::Stable(engine) => engine.state(),
        }
    }
}

pub use actor::{
    Authorization, authorize, init, install, install_with_login_limit, refresh, with_authorization,
};

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
use std::cell::RefCell;

struct State {
    enrollment: enrollment::Receiver,
    receiver: Engine,
    policy: Policy,
    max_login_ttl_ns: u64,
}

thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

pub(crate) fn configured_issuer() -> Option<Principal> {
    STATE.with_borrow(|slot| slot.as_ref().map(|state| state.policy.issuer))
}

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

pub(super) fn initialize(memory: MemoryConfig, policy: Policy, own_id: Principal, now: u64) {
    initialize_with(memory, policy, own_id, now, |config| {
        Engine::Volatile(MemoryReceiver::new_fresh(config, now).expect("fresh session receiver"))
    });
}

fn initialize_with(
    memory: MemoryConfig,
    policy: Policy,
    own_id: Principal,
    now: u64,
    build: impl FnOnce(MemoryConfig) -> Engine,
) {
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
    let receiver = build(memory);
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

fn view(receipt: SessionReceipt) -> SessionView {
    SessionView {
        ext: receipt.ext.map(|bytes| {
            ic_wire::Bytes::new(bytes).expect("verified Auth extension within protocol limit")
        }),
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
        s.receiver.state().prune(now, 8).map_err(admission_error)?;
        let outcome = s
            .receiver
            .state()
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
    authorized(caller, now, scope, |receipt| view(receipt.clone()))
}

fn authorized<R>(
    caller: Principal,
    now: u64,
    scope: &str,
    project: impl FnOnce(&SessionReceipt) -> R,
) -> Result<R, SessionReadRefusal> {
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
            .state()
            .authorize(
                &authority,
                &[scope
                    .parse()
                    .map_err(|_| SessionReadRefusal::ScopeRejected)?],
                project,
            )
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

        match s.receiver.state().logout(caller, now) {
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

/// Mount the standard receiver and its controller-only refresh.
/// The application selects the read endpoint's scope and explicitly initializes on install.
#[macro_export]
macro_rules! mount_auth_session {
    (read = $scope:expr $(,)?) => {
        const _: () = {
            use $crate::auth::{
                enrollment::types::Principal,
                protocol::{SessionView, model::SessionReadRefusal},
                session,
            };

            fn __canic_session_read(
                caller: Principal,
                now: u64,
            ) -> Result<SessionView, SessionReadRefusal> {
                session::read_scope(caller, now, $scope)
            }

            $crate::mount_auth_endpoints! {
                admit = $crate::auth::session::admit,
                read = __canic_session_read,
                logout = $crate::auth::session::logout,
            }

            $crate::mount_auth_refresh_endpoint!($crate::auth::session::refresh);
        };
    };
}

/// Mount install and upgrade hooks for the selected receiver storage.
/// With `storage = open`, the application returns its admitted storage directory.
/// Stable installs require both receiver leases and seal their state before upgrade.
/// Volatile installs omit both leases and stay closed after upgrade.
/// The `decode` and `open` form supports volatile-only receivers.
/// Optional `max_login_secs` sets the absolute login ceiling. Omitting it uses
/// the IC Auth starting profile. Proof and receiver-session TTLs are independent.
#[macro_export]
macro_rules! mount_auth_lifecycle {
    (storage = $open:path $(, max_login_secs = $seconds:expr)?
     $(, init = $init:path)? $(, pre_upgrade = $pre:path)? $(, post_upgrade = $post:path)? $(,)?) => {
        const _: () = {
            use $crate::__private::runtime;
            use $crate::auth::{endpoint::fail, install::Install, session::stable};
            #[allow(clippy::needless_pass_by_value)]
            fn __canic_install(input: Install) {
                stable::open_with_login_limit(&$open(Some(&input)), Some(&input), $crate::mount_auth_lifecycle!(@limit $($seconds)?));
                $( $init(&input); )?
            }
            fn __canic_reopen() {
                stable::open_with_login_limit(&$open(None), None, $crate::mount_auth_lifecycle!(@limit $($seconds)?));
                $( $post(); )?
            }
            $crate::mount_lifecycle!(input = Install, max_bytes = 4096, decode = Install::decode,
                install = __canic_install, reopen = __canic_reopen);
            fn admit() -> Result<(), std::convert::Infallible> { Ok(()) }
            use $crate::lifecycle::empty_arguments;
            #[runtime::pre_upgrade(runtime = runtime, guard = admit,
                decode = empty_arguments, max_bytes = 0, on_error = fail)]
            fn __canic_pre_upgrade((): ()) { stable::seal(); $( $pre(); )? }
        };
    };
    (decode = $decode:path, open = $open:path $(, max_login_secs = $seconds:expr)? $(,)?) => {
        const _: () = {
            use $crate::auth::{endpoint::fail, install::Install, session};

            $crate::mount_lifecycle!(
                input = Install,
                max_bytes = 4096,
                decode = Install::decode,
                install = __canic_install_receiver,
                reopen = __canic_reopen_application,
            );

            #[allow(clippy::needless_pass_by_value)]
            fn __canic_install_receiver(input: Install) {
                let application = $decode(&input).unwrap_or_else(|error| fail(error));
                session::install_with_login_limit(&input, $crate::mount_auth_lifecycle!(@limit $($seconds)?));
                $open(Some(application));
            }

            fn __canic_reopen_application() { $open(None); }
        };
    };
    (@limit) => { None };
    (@limit $seconds:expr) => { Some($seconds) };
}
