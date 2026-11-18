//! Application-owned, volatile test signer. No production key custody is claimed.
use crate::{IssuerObservation, audience};
use canic::auth::issuer::{Issue, Logout, Lookup, Refusal, Renew, Revoked, Status};
use canic::auth::protocol::wire as auth_wire;
use ic_auth::{
    issuer_memory::{
        IssueRequest, IssuedProof, IssuerAuthority, IssuerConfig, IssuerError, IssuerPolicy,
        IssuerSigning, IssuerStatus, IssuerTicket, MemoryIssuer, RenewRequest,
    },
    verification::Ed25519Key,
};
use ic_auth_protocol_types::{DelegatedRoleGrant, Principal};
use ic_platform_crypto::ed25519::{Signer, SigningKey};
use ic_platform_protocol::raw_rand;
use ic_platform_runtime::{
    self as runtime,
    call::{Call, Outcome, Wait},
    system as api,
};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

const SECOND: u64 = 1_000_000_000;
const POLICY: IssuerPolicy = IssuerPolicy {
    login_ttl_ns: 300 * SECOND,
    proof_ttl_ns: 60 * SECOND,
    receipt_timeout_ns: 30 * SECOND,
    reconciliation_retention_ns: 60 * SECOND,
};
struct State {
    engine: MemoryIssuer,
    account: Principal,
    authority: IssuerAuthority,
    signer: SigningKey,
    allowed: bool,
    signatures: u32,
    reply_fault: bool,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
pub(crate) fn init() {
    let now = api::time();
    let signer = SigningKey::from_bytes(&[7; 32]);
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            engine: MemoryIssuer::new_fresh(
                IssuerConfig {
                    audience: audience(),
                    issuer: api::canister_self(),
                    max_logins: if cfg!(feature = "profile") { 5 } else { 2 },
                    max_operations: if cfg!(feature = "profile") { 8 } else { 4 },
                    max_operations_per_account: 4,
                    // Replacement retains revoked login evidence; it does not reclaim its slot.
                    max_devices_per_account: if cfg!(feature = "profile") { 3 } else { 2 },
                    max_result_bytes: 16384,
                    issuance_burst: 8,
                    issuance_refill_interval_ns: SECOND,
                },
                now,
            )
            .unwrap(),
            account: if cfg!(feature = "profile") {
                ic_auth::verification::browser_principal(
                    &SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes(),
                )
            } else {
                Principal::self_authenticating(b"canic-auth-account")
            },
            authority: IssuerAuthority {
                signing: IssuerSigning::Ed25519,
                generation: 1,
                key_binding_id: Ed25519Key::new(signer.verifying_key().to_bytes())
                    .unwrap()
                    .binding(),
                expires_at_ns: now + 600 * SECOND,
            },
            signer,
            allowed: true,
            signatures: 0,
            reply_fault: false,
        })
    });
}
// No upgrade initializer: missing volatile intent/replay state stays unavailable.
pub(crate) fn control(command: u8) -> bool {
    STATE.with_borrow_mut(|state| {
        let Some(s) = state else {
            return false;
        };
        // The caller is already checked by the shared installation-owned fixture control.
        match command {
            6 => s.reply_fault = true,
            7 => s.reply_fault = false,
            8 => s.allowed = false,
            9 => {
                s.engine.prune(api::time(), 32).unwrap();
            }
            10 if s.authority.generation == 1 => {
                s.signer = SigningKey::from_bytes(&[17; 32]);
                s.authority.key_binding_id = Ed25519Key::new(s.signer.verifying_key().to_bytes())
                    .unwrap()
                    .binding();
                s.authority.generation = 2;
            }
            _ => return false,
        }
        true
    })
}
fn refused(error: IssuerError) -> Refusal {
    match error {
        IssuerError::Invalid => Refusal::Invalid,
        IssuerError::Denied => Refusal::Denied,
        IssuerError::Conflict | IssuerError::EpochClosed => Refusal::Conflict,
        IssuerError::Expired => Refusal::Expired,
        IssuerError::Capacity => Refusal::Capacity,
        IssuerError::Throttled => Refusal::Throttled,
        IssuerError::Pending
        | IssuerError::Dispatched
        | IssuerError::Clock
        | IssuerError::Closed => Refusal::Unavailable,
    }
}
fn grants(
    input: auth_wire::List<canic::auth::protocol::model::DelegatedRoleGrant, 16>,
) -> Result<Vec<DelegatedRoleGrant>, Refusal> {
    input
        .into_vec()
        .into_iter()
        .map(|g| {
            Ok(DelegatedRoleGrant {
                target: g.target.as_str().parse().map_err(|_| Refusal::Invalid)?,
                scopes: g
                    .scopes
                    .as_slice()
                    .iter()
                    .map(|s| s.as_str().parse().map_err(|_| Refusal::Invalid))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect()
}
fn known_account(s: &State, account: Principal) -> bool {
    account == s.account
        || (cfg!(feature = "profile")
            && account
                == ic_auth::verification::browser_principal(
                    &SigningKey::from_bytes(&[10; 32]).verifying_key().to_bytes(),
                ))
}
fn permit(s: &State, account: Principal, grants: &[DelegatedRoleGrant]) -> Result<(), Refusal> {
    if !s.allowed
        || !known_account(s, account)
        || grants.len() != 1
        || grants[0].target.as_str() != "user"
        || grants[0].scopes.is_empty()
        || grants[0].scopes.iter().any(|scope| {
            scope.as_str() != "read" && !(cfg!(feature = "profile") && scope.as_str() == "write")
        })
    {
        return Err(Refusal::Denied);
    }
    Ok(())
}
fn renewal(input: Renew) -> Result<RenewRequest, Refusal> {
    Ok(RenewRequest {
        account: input.account,
        login_epoch: input.login_epoch,
        login_id: input.login_id,
        sequence: input.sequence,
        grants: grants(input.grants)?,
    })
}
fn status(value: Option<IssuerStatus<'_>>) -> Status {
    let Some(IssuerStatus::Completed(result)) = value else {
        return Status::Unresolved;
    };
    let IssuedProof::Ed25519(signature) = result.proof else {
        api::trap("test signer proof mismatch");
    };
    // Any representation failure after mutation traps the whole callback.
    Status::Issued {
        credential: canic::auth::protocol::model::PortableCredential {
            claims: auth_wire::Bytes::new(result.claims.to_vec()).expect("bounded supplier claims"),
            proof: canic::auth::protocol::model::PortableProof::Ed25519(
                auth_wire::Bytes::new(signature.to_vec()).unwrap(),
            ),
        },
        admission_deadline_ns: result.admission_deadline_ns,
    }
}
fn retained(s: &State, ticket: IssuerTicket) -> Result<Status, Refusal> {
    s.engine
        .reconcile(ticket.account(), ticket.epoch(), ticket.operation_id())
        .map(status)
        .map_err(refused)
}
async fn issue(caller: Principal, input: Issue, now: u64) -> Result<Status, Refusal> {
    let request = IssueRequest {
        epoch: input.epoch,
        operation_id: input.operation_id,
        login_epoch: input.login_epoch,
        login_id: input.login_id,
        browser_public_key: input.browser_public_key,
        grants: grants(input.grants)?,
    };
    let (ticket, existing) = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(Refusal::Unavailable)?;
        permit(s, caller, &request.grants)?;
        let existing = s
            .engine
            .reconcile(caller, request.epoch, request.operation_id)
            .map_err(refused)?
            .is_some();
        let ticket = s
            .engine
            .prepare_issue(caller, &request, POLICY, s.authority, now)
            .map_err(refused)?;
        Ok::<_, Refusal>((ticket, existing))
    })?;
    finish::<{ canic::auth::issuer::ISSUE }>(ticket, existing).await
}
async fn renew(caller: Principal, input: Renew, now: u64) -> Result<Status, Refusal> {
    let request = renewal(input)?;
    let (ticket, existing) = STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(Refusal::Unavailable)?;
        let selection = s
            .engine
            .preflight_renewal(caller, &request)
            .map_err(refused)?;
        permit(s, selection.account, &selection.request.grants)?;
        let existing = s
            .engine
            .reconcile_renewal(caller, &request)
            .map_err(refused)?
            .is_some();
        let ticket = s
            .engine
            .prepare_renewal(caller, &request, s.authority, now)
            .map_err(refused)?;
        Ok::<_, Refusal>((ticket, existing))
    })?;
    finish::<{ canic::auth::issuer::RENEW }>(ticket, existing).await
}
async fn finish<const ID: u64>(ticket: IssuerTicket, existing: bool) -> Result<Status, Refusal> {
    if existing {
        return STATE
            .with_borrow(|state| retained(state.as_ref().ok_or(Refusal::Unavailable)?, ticket));
    }
    // Reservation and outgoing entropy call commit together before any signing.
    // Duplicate or abandoned work is lookup-only, including after a callback trap.
    let response = Call::new(
        Principal::management_canister(),
        raw_rand::METHOD,
        Wait::Bounded { seconds: 300 },
    )
    .args(raw_rand::encode(6).map_err(|_| Refusal::Unavailable)?)
    .execute()
    .await
    .map_err(|_| Refusal::Unavailable)?;
    let response = match response.into_parts().0 {
        Outcome::Reply(bytes) => bytes,
        _ => return Err(Refusal::Unavailable),
    };
    let randomness = raw_rand::decode(response.as_ref(), 42).map_err(|_| Refusal::Unavailable)?;
    let nonce: [u8; 16] = randomness[..16].try_into().unwrap();
    if nonce == [0; 16] {
        return Err(Refusal::Unavailable);
    }
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(Refusal::Unavailable)?;
        let now = api::time();
        let intent = s.engine.intent(ticket).map_err(refused)?;
        permit(s, ticket.account(), intent.grants)?;
        s.engine
            .record_nonce(ticket, s.authority, nonce, now)
            .map_err(refused)?;
        let mut commitment = Sha256::new();
        commitment.update(b"canic-issuer-witness-approved");
        commitment.update(ticket.operation_id());
        commitment.update(nonce);
        s.engine
            .record_approval(ticket, s.authority, commitment.finalize().into(), now)
            .map_err(refused)?;
        let signing = s
            .engine
            .dispatch(ticket, s.authority, now)
            .map_err(refused)?;
        // Local test-key signing has no independently committed external effect.
        // Signature, result, counter and reply encoding share this IC transaction.
        let signature = s.signer.sign(&Sha256::digest(signing.claims())).to_bytes();
        s.engine
            .complete(ticket, s.authority, signature, now)
            .expect("atomic local signing completion");
        s.signatures += 1;
        let result = retained(s, ticket).expect("completed local signing result");
        crate::canister::reply_fault::<ID>(s.reply_fault);
        Ok(result)
    })
}
fn reconcile(caller: Principal, input: Lookup, _: u64) -> Result<Status, Refusal> {
    STATE.with_borrow(|state| {
        let s = state.as_ref().ok_or(Refusal::Unavailable)?;
        match input {
            Lookup::Issue {
                epoch,
                operation_id,
            } => {
                if !known_account(s, caller) {
                    return Err(Refusal::Denied);
                }
                s.engine
                    .reconcile(caller, epoch, operation_id)
                    .map(status)
                    .map_err(refused)
            }
            Lookup::Renew(request) => s
                .engine
                .reconcile_renewal(caller, &renewal(request)?)
                .map(|retained| status(retained.map(|r| r.status)))
                .map_err(refused),
        }
    })
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "the mounted handler owns its decoded request"
)]
fn logout(caller: Principal, input: Logout, now: u64) -> Result<Revoked, Refusal> {
    STATE.with_borrow_mut(|state| {
        let s = state.as_mut().ok_or(Refusal::Unavailable)?;
        let login_deadline_ns = s
            .engine
            .revoke_browser(
                caller,
                input.account,
                input.login_epoch,
                input.login_id,
                now,
            )
            .map_err(refused)?;
        crate::canister::reply_fault::<{ canic::auth::issuer::LOGOUT }>(s.reply_fault);
        Ok(Revoked { login_deadline_ns })
    })
}
canic::mount_auth_issuer_endpoints! {
    issue = super::issue, renew = super::renew, reconcile = super::reconcile,
    logout = super::logout,
}

#[runtime::query(runtime = runtime, guard = lifecycle_guard, decode = crate::canister::decode_observe, max_bytes = 2097152, on_error = endpoint_error, name = "observe_issuer", encode = crate::canister::encode_observe::<IssuerObservation>)]
fn observe_issuer((): ()) -> IssuerObservation {
    STATE.with_borrow(|state| match state {
        None => IssuerObservation {
            present: false,
            signatures: 0,
            operations: 0,
            logins: 0,
            epoch: 0,
        },
        Some(s) => IssuerObservation {
            present: true,
            signatures: s.signatures,
            operations: s.engine.usage().operations,
            logins: s.engine.usage().logins,
            epoch: s.engine.epoch(),
        },
    })
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}
