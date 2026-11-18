//! Explicit Root enrollment around the real volatile engine.
use crate::{LIMITS, OBSERVE, Observation, RefreshObservation, audience, enrollment, protocol};
use canic::auth::protocol::wire as auth_wire;
use ic_auth::{
    receiver_memory::{
        AdmissionHost, EnrollmentBinding, LoginLimits, LogoutError, LogoutReceipt, MemoryConfig,
        MemoryError, MemoryReceiver, SessionAuthority, SessionCapacity, SessionPolicyError,
        SessionReceipt, UnknownLogoutLimits,
    },
    session_request::{SessionRequest, SessionTtlPolicy},
    verification::{Ed25519Key, VerificationContext, VerificationCosts, VerificationProfile},
};
use ic_auth_protocol_types::{PortableMechanism, Principal};
use ic_platform_runtime::{self as runtime, raw, system as api};
use protocol::{
    SessionInput, SessionView,
    model::{AdmissionRefusal, LogoutResult, LogoutUnavailable, SessionReadRefusal},
};
use serde::{Serialize, Serializer};
use std::cell::RefCell;

const SECOND: u64 = 1_000_000_000;
struct State {
    receiver: MemoryReceiver,
    installer: Principal,
    enrollment: enrollment::Receiver,
    calls: u32,
    writes: u32,
    fault: bool,
    enabled: bool,
    refresh: RefreshObservation,
    refresh_barrier: Option<Principal>,
}

thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

fn decode_init(bytes: &[u8]) -> Result<Principal, std::convert::Infallible> {
    Ok({ Principal::try_from_slice(bytes).expect("installed authority principal") })
}

/// Initialize the auth witness state for this installation.
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = decode_init,
    max_bytes = 2097152,
    on_error = endpoint_error,
)]
fn init(authority: Principal) {
    #[cfg(feature = "profile")]
    crate::profile::open(true);
    let now = api::time();
    let installer = api::caller();
    crate::issuer::init();
    let receiver = MemoryReceiver::new_fresh(
        MemoryConfig {
            audience: audience(),
            enrollment: EnrollmentBinding {
                receiver: api::canister_self(),
                authority,
            },
            login: LoginLimits {
                max_device_slots: 8,
                max_login_ttl_ns: 300 * SECOND,
                max_admitted_logins: if cfg!(feature = "profile") { 4 } else { 2 },
            },
            capacity: SessionCapacity {
                max_sessions: if cfg!(feature = "profile") { 4 } else { 2 },
                max_replays: if cfg!(feature = "profile") { 8 } else { 4 },
                max_replays_per_account: 4,
                max_encoded_bytes: 8192,
            },
            verification_capacity: 20,
            refill_interval_ns: SECOND,
            verification_costs: VerificationCosts {
                ed25519: 2,
                certified: 3,
            },
            unknown_logout: UnknownLogoutLimits {
                max_denials: 2,
                capacity: 2,
                refill_interval_ns: SECOND,
            },
        },
        now,
    )
    .unwrap();
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            receiver,
            installer,
            enrollment: enrollment::Receiver::new(
                EnrollmentBinding {
                    receiver: api::canister_self(),
                    authority,
                },
                audience(),
                now,
            )
            .unwrap(),
            calls: 0,
            writes: 0,
            fault: false,
            enabled: true,
            refresh: RefreshObservation::default(),
            refresh_barrier: None,
        })
    });
}
// No post-upgrade initializer: volatile safety loss remains closed, including to
// the fixture's enrollment command. It cannot reconstruct replay/revocation state.

// Qualification-only pause after the real Root reply, before local publication.
// The barrier supplies no snapshot or authentication marker.
/// Let the installer hold receiver refresh at the selected fixture barrier.
#[runtime::update(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = decode_init,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "set_refresh_barrier",
    encode = encode_control,
)]
fn set_refresh_barrier(barrier: Principal) -> bool {
    STATE.with_borrow_mut(|state| {
        let Some(s) = state else { return false };
        if api::caller() != s.installer || barrier == Principal::anonymous() {
            return false;
        }
        s.refresh_barrier = (barrier != Principal::management_canister()).then_some(barrier);
        true
    })
}

/// Require a single command byte. The handler checks installer authority.
fn control_guard() -> Result<(), String> {
    if raw::msg_arg_data_size() == 1 {
        Ok(())
    } else {
        Err("fixture command length".into())
    }
}

fn decode_control(bytes: &[u8]) -> Result<u8, std::convert::Infallible> {
    Ok(bytes[0])
}

fn encode_control(value: bool) -> Result<Vec<u8>, std::convert::Infallible> {
    Ok({ vec![u8::from(value)] })
}

/// Apply the installer's selected Auth witness control.
#[runtime::update(
    runtime = runtime,
    guard = control_guard,
    decode = decode_control,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "control",
    encode = encode_control,
)]
fn control(command: u8) -> bool {
    STATE.with_borrow_mut(|state| {
        let Some(s) = state else {
            return false;
        };
        if api::caller() != s.installer {
            return false;
        }
        match command {
            2 => s.fault = true,
            3 => s.fault = false,
            4 => s.enabled = false,
            5 => s.enabled = true,
            _ => return crate::issuer::control(command),
        }
        true
    })
}

canic::mount_auth_refresh_endpoint!(super::refresh);

async fn refresh() -> Result<(), enrollment::Error> {
    let start = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;

        let now = api::time();
        let start = s.enrollment.begin(now)?;
        s.refresh.reserved += 1;
        s.refresh.deadline_ns = now + enrollment::RECEIPT_NS;
        Ok(start)
    })?;
    let challenge = start.entropy().await?;
    let pull = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;
        let pull = s
            .enrollment
            .prepare(&mut s.receiver, challenge, api::time())?;
        s.refresh.dispatched += 1;
        Ok(pull)
    })?;
    let response = pull.fetch().await?;
    let barrier = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;
        s.refresh.fetched += 1;
        Ok(s.refresh_barrier)
    })?;
    if let Some(barrier) = barrier {
        let reply = runtime::call::Call::new(
            barrier,
            "hold",
            runtime::call::Wait::Bounded { seconds: 300 },
        )
        .args(&[])
        .execute()
        .await
        .map_err(|_| enrollment::Error::Unavailable)?;
        if !matches!(reply.into_parts().0, runtime::call::Outcome::Reply(bytes) if bytes.is_empty())
        {
            return Err(enrollment::Error::Unavailable);
        }
    }
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;
        s.enrollment
            .finish(&mut s.receiver, response, api::time())?;
        s.refresh.installed += 1;
        if s.fault {
            api::trap("after supplier metadata and host entry publication");
        }
        Ok(())
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
        MemoryError::Claims(_) | MemoryError::Context(_) | MemoryError::Proof(_) => {
            AdmissionRefusal::InvalidCredential
        }
        MemoryError::Budget(_) => AdmissionRefusal::Throttled,
    }
}
// Invoke the production encoder with a deliberately failing serializer after
// real engine/application mutation. No production fault switch or alternate mount.
pub(crate) struct Unencodable;
impl Serialize for Unencodable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "qualification reply encoding fault",
        ))
    }
}

pub(crate) fn reply_fault<const ID: u64>(enabled: bool) {
    if enabled {
        let _ = canic::auth::endpoint::encode::<_, ID>(Unencodable).unwrap_or_else(|error| {
            api::trap(&format!("qualification reply encoding fault: {error:?}"))
        });
    }
}

#[cfg(feature = "profile")]
pub(crate) fn profile_fault() -> bool {
    STATE.with_borrow(|state| state.as_ref().is_some_and(|state| state.fault))
}

fn admit(
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
            .map(auth_wire::Text::into_string)
            .collect(),
        input.ttl_secs,
    )
    .map_err(|_| AdmissionRefusal::ScopeRejected)?;
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(AdmissionRefusal::Unavailable)?;
        s.calls += 1;
        let snapshot = s
            .enrollment
            .current(now)
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        let audience = snapshot.grant.audience();
        let grants = snapshot.grant.grants().unwrap();
        let role = snapshot.grant.grant.target.as_str().parse().unwrap();
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
            max_login_ttl_ns: 300 * SECOND,
            max_proof_ttl_ns: 60 * SECOND,
        };
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
                    role: &role,
                    ttl: SessionTtlPolicy::new(60, 1800).unwrap(),
                    generation: snapshot.generation,
                    enabled: s.enabled && !snapshot.revoked,
                    subject_admissible: true,
                },
            )
            .map_err(admission_error)?;
        if !outcome.exact_retry {
            s.writes += 1;
        }
        reply_fault::<{ protocol::EstablishSession::ID }>(s.fault);
        Ok(view(outcome.receipt))
    })
}

fn read(caller: Principal, now: u64) -> Result<SessionView, SessionReadRefusal> {
    read_scope(caller, now, "read")
}

pub(crate) fn read_scope(
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
        let grants = snapshot.grant.grants().unwrap();
        let role = snapshot.grant.grant.target.as_str().parse().unwrap();
        let authority = SessionAuthority {
            caller,
            now_ns: now,
            audience: snapshot.grant.audience(),
            issuer: snapshot.grant.issuer,
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: Ed25519Key::new(snapshot.grant.key).unwrap().binding(),
            role: &role,
            grants: grants.as_slice(),
            generation: snapshot.generation,
            not_before_ns: snapshot.grant.not_before_ns,
            expires_at_ns: snapshot.grant.expires_at_ns,
            enrollment_expires_at_ns: snapshot.expires_at_ns,
            min_login_started_at_ns: 0,
            max_session_ttl_secs: 1800,
            enabled: s.enabled && !snapshot.revoked,
            subject_admissible: true,
        };
        s.receiver
            .read(&authority, &[scope.parse().unwrap()])
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

fn logout(caller: Principal, now: u64) -> LogoutResult {
    STATE.with_borrow_mut(|state| {
        let Some(s) = state else {
            return LogoutResult::Unavailable(LogoutUnavailable::Storage);
        };
        s.calls += 1;
        let result = match s.receiver.logout(caller, now) {
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
        };
        reply_fault::<{ protocol::Logout::ID }>(s.fault);
        result
    })
}

canic::mount_auth_endpoints! {
    admit = super::admit,
    read = super::read,
    logout = super::logout,
}

/// Read the receiver's current enrollment evidence.
#[runtime::query(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = decode_observe,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "observe_enrollment",
    encode = enrollment::transport::encode::<Result<enrollment::Snapshot, enrollment::Error>, {enrollment::SNAPSHOT}>,
)]
fn observe_enrollment((): ()) -> Result<enrollment::Snapshot, enrollment::Error> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .ok_or(enrollment::Error::Closed)?
            .enrollment
            .current(api::time())
            .cloned()
    })
}

pub(crate) fn decode_observe(bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    assert!(bytes.is_empty());
    Ok(())
}

pub(crate) fn encode_observe<T: Serialize>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

/// Read the receiver's fixture state and call counts.
#[runtime::query(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = decode_observe,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "observe",
    encode = encode_observe::<Observation>,
)]
fn observe((): ()) -> Observation {
    STATE.with_borrow(|state| {
        let Some(s) = state else {
            return Observation {
                present: false,
                calls: 0,
                writes: 0,
                usage: (0, 0, 0),
                credit: 0,
                enrollment: None,
            };
        };
        let usage = s
            .receiver
            .usage((Principal::from_slice(&[1]), Principal::from_slice(&[2])));
        Observation {
            present: true,
            calls: s.calls,
            writes: s.writes,
            usage: (
                usage.sessions as u64,
                usage.replays as u64,
                usage.encoded_bytes as u64,
            ),
            credit: s.receiver.verification_budget().available(),
            enrollment: s.receiver.enrollment().map(|m| (m.revision, m.generation)),
        }
    })
}

/// Read the witness's recorded refresh progress.
#[runtime::query(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = decode_observe,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "observe_refresh",
    encode = encode_observe::<RefreshObservation>,
)]
fn observe_refresh((): ()) -> RefreshObservation {
    STATE.with_borrow(|state| state.as_ref().expect("original fixture must exist").refresh)
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

/// The IC admits lifecycle calls. The declared argument budget still applies.
fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}
