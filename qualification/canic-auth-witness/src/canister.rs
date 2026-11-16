//! Explicit Root enrollment around the real volatile engine.
use crate::{LIMITS, OBSERVE, Observation, audience, enrollment, protocol};
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
use ic_platform_runtime::{cdk, cdk::api, ic0};
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
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers the installation buffer"
)]
fn decode_init(bytes: Vec<u8>) -> Principal {
    Principal::try_from_slice(&bytes).expect("installed authority principal")
}
#[cdk::init(decode_with = "decode_init")]
fn init(authority: Principal) {
    let now = api::time();
    let installer = api::msg_caller();
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
                max_admitted_logins: 2,
            },
            capacity: SessionCapacity {
                max_sessions: 2,
                max_replays: 4,
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
        })
    });
}
// No post-upgrade initializer: volatile safety loss remains closed, including to
// the fixture's enrollment command. It cannot reconstruct replay/revocation state.

fn control_guard() -> Result<(), String> {
    if ic0::msg_arg_data_size() == 1 {
        Ok(())
    } else {
        Err("fixture command length".into())
    }
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers argument ownership"
)]
fn decode_control(bytes: Vec<u8>) -> u8 {
    bytes[0]
}
fn encode_control(value: bool) -> Vec<u8> {
    vec![u8::from(value)]
}
#[cdk::update(
    guard = "control_guard",
    decode_with = "decode_control",
    encode_with = "encode_control"
)]
async fn control(command: u8) -> bool {
    if command == 1 {
        return refresh().await.is_ok();
    }
    STATE.with_borrow_mut(|state| {
        let Some(s) = state else {
            return false;
        };
        if api::msg_caller() != s.installer {
            return false;
        }
        match command {
            2 => s.fault = true,
            3 => s.fault = false,
            4 => s.enabled = false,
            5 => s.enabled = true,
            _ => return false,
        }
        true
    })
}
async fn refresh() -> Result<(), enrollment::Error> {
    let start = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;
        if api::msg_caller() != s.installer {
            return Err(enrollment::Error::Binding);
        }
        s.enrollment.begin(api::time())
    })?;
    let challenge = start.entropy().await?;
    let pull = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;
        s.enrollment
            .prepare(&mut s.receiver, challenge, api::time())
    })?;
    let response = pull.fetch().await?;
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(enrollment::Error::Closed)?;
        s.enrollment
            .finish(&mut s.receiver, response, api::time())?;
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
struct Unencodable;
impl Serialize for Unencodable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "qualification reply encoding fault",
        ))
    }
}
fn reply_fault<const ID: u64>(enabled: bool) {
    if enabled {
        let _ = canic::auth::endpoint::encode::<_, ID>(Unencodable);
    }
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
            .map(ic_wire::Text::into_string)
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
            .read(&authority, &["read".parse().unwrap()])
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
canic::mount_auth_endpoints! { admit = super::admit, read = super::read, logout = super::logout }

#[cdk::query(
    decode_with = "decode_observe",
    encode_with = "enrollment::transport::encode::<Result<enrollment::Snapshot, enrollment::Error>, {enrollment::SNAPSHOT}>"
)]
fn observe_enrollment() -> Result<enrollment::Snapshot, enrollment::Error> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .ok_or(enrollment::Error::Closed)?
            .enrollment
            .current(api::time())
            .cloned()
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers argument ownership"
)]
fn decode_observe(bytes: Vec<u8>) {
    assert!(bytes.is_empty());
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers response ownership"
)]
fn encode_observe(value: Observation) -> Vec<u8> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}
#[cdk::query(decode_with = "decode_observe", encode_with = "encode_observe")]
fn observe() -> Observation {
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
