#![expect(clippy::unused_async)]
use candid::{CandidType, Principal};
use canic::access::{AccessCheck, AccessContext, AccessError};
use canic::endpoint::ArgumentLimits;
use canic::{Error, prelude::*};
use std::cell::Cell;

const LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 10_000,
    skipping_quota: 32,
    max_type_len: 8,
    max_header_len: 128,
};
const LIFECYCLE_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 4096,
    decoding_quota: 20_000,
    skipping_quota: 64,
    max_type_len: 16,
    max_header_len: 256,
};
const WORK_LIMITS: ArgumentLimits = ArgumentLimits {
    decoding_quota: 40,
    ..LIMITS
};
const TYPE_LIMITS: ArgumentLimits = ArgumentLimits {
    max_type_len: 1,
    ..LIMITS
};
const HEADER_LIMITS: ArgumentLimits = ArgumentLimits {
    max_header_len: 8,
    ..LIMITS
};

thread_local! {
    static HANDLER_CALLS: Cell<u64> = const { Cell::new(0) };
    static PREDICATE_CALLS: Cell<u64> = const { Cell::new(0) };
}

canic::start_local!(
    argument_limits = LIFECYCLE_LIMITS,
    lifecycle_participant(init = bounded_init, post_upgrade = bounded_post_upgrade,),
);

fn bounded_init() {
    ic_cdk::api::debug_print("bounded participant init");
    // Prove that participant witnesses survive a failed install, even though
    // reinstall clears the previous canister logs before entering init.
    let (args,): (Option<Vec<u8>>,) = LIFECYCLE_LIMITS
        .decode(&ic_cdk::api::msg_arg_data())
        .expect("the framework already admitted this envelope");
    if args.as_deref() == Some(&[1]) {
        ic_cdk::api::debug_print("bounded participant trap witness");
        ic_cdk::trap("participant witness");
    }
}
fn bounded_post_upgrade() {
    ic_cdk::api::debug_print("bounded participant post_upgrade");
}

/// A plain protocol record, with no Result envelope on the wire.
#[derive(CandidType)]
struct PlainReply {
    committed: u64,
    predicates: u64,
}

struct Gate(bool);

impl AccessCheck for Gate {
    fn eval(&self, _: &AccessContext) -> Result<(), AccessError> {
        // Test-only observations distinguish short-circuiting from handler dispatch.
        PREDICATE_CALLS.set(PREDICATE_CALLS.get() + 1);
        if self.0 {
            Ok(())
        } else {
            Err(AccessError::ControllerRequired)
        }
    }
    fn name(&self) -> &'static str {
        "probe_gate"
    }
}

fn commit() -> PlainReply {
    ic_cdk::api::debug_print("bounded handler dispatched");
    HANDLER_CALLS.set(HANDLER_CALLS.get() + 1);
    PlainReply {
        committed: HANDLER_CALLS.get(),
        predicates: PREDICATE_CALLS.get(),
    }
}

#[canic_update(requires(custom(Gate(allow)), custom(Gate(true))), on_access_denied = "reject", decode = LIMITS)]
fn plain_commit(allow: bool) -> PlainReply {
    assert!(allow);
    commit()
}

#[canic_update(requires(custom(Gate(allow)), custom(Gate(true))), decode = LIMITS)]
fn result_commit(allow: bool) -> Result<PlainReply, Error> {
    assert!(allow);
    Ok(commit())
}

#[canic_query(public, on_access_denied = "reject", decode = LIMITS)]
fn dispatch_counts() -> PlainReply {
    PlainReply {
        committed: HANDLER_CALLS.get(),
        predicates: PREDICATE_CALLS.get(),
    }
}

#[canic_query(public)]
fn access_counts() -> Result<Vec<canic::dto::metrics::MetricEntry>, Error> {
    Ok(
        canic::api::metrics::MetricsQuery::security(canic::dto::page::PageRequest {
            limit: 100,
            offset: 0,
        })
        .entries,
    )
}

#[canic_query(public, on_access_denied = "reject", decode = LIMITS)]
fn bounded_query(value: String) -> PlainReply {
    drop(value);
    commit()
}

#[canic_update(public, on_access_denied = "reject", decode = LIMITS)]
fn bounded_update(value: String) -> PlainReply {
    drop(value);
    commit()
}

#[canic_update(public, on_access_denied = "reject", decode = WORK_LIMITS)]
fn bounded_work(value: Vec<()>) -> PlainReply {
    drop(value);
    commit()
}

#[canic_update(public, on_access_denied = "reject", decode = LIMITS)]
fn bounded_skip() -> PlainReply {
    commit()
}

#[canic_update(public, on_access_denied = "reject", decode = TYPE_LIMITS)]
fn bounded_types(value: Vec<u8>) -> PlainReply {
    drop(value);
    commit()
}

#[canic_update(public, on_access_denied = "reject", decode = HEADER_LIMITS)]
fn bounded_header(value: u64) -> PlainReply {
    let _ = value;
    commit()
}

/// Relay arbitrary bytes so update refusal is exercised without inspect_message.
#[canic_update(public, payload(max_bytes = 16 * 1024))]
async fn relay_bounded(target: Principal, method: String, bytes: Vec<u8>) -> Result<bool, Error> {
    Call::unbounded_wait(target, &method)
        .with_raw_args(bytes)
        .execute()
        .await
        .map(|_| true)
}

// Provide an empty setup hook so `start!` can schedule user lifecycle work.
async fn canic_setup() {}

// Provide an empty install hook; payload tests only exercise update ingress.
async fn canic_install(_: Option<Vec<u8>>) {}

// Provide an empty upgrade hook for the required Canic lifecycle surface.
async fn canic_upgrade() {}

/// Relay a generated payload through a real inter-canister update.
#[canic_update(public)]
async fn relay_explicit_echo(target: Principal, len: usize) -> Result<usize, Error> {
    Call::unbounded_wait(target, "explicit_echo")
        .with_arg("x".repeat(len))?
        .execute_candid::<Result<usize, Error>>()
        .await?
}

/// Echo payload length under the default update ingress limit.
#[canic_update(public)]
fn default_echo(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}

/// Echo payload length under an explicit larger update ingress limit.
#[canic_update(public, payload(max_bytes = 32 * 1024))]
fn explicit_echo(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}

/// Echo payload length under an explicit limit and exported method name.
#[canic_update(public, name = "wire_named_echo", payload(max_bytes = 24 * 1024))]
fn named_echo(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}

/// Bare CDK updates still inherit the managed ingress inspector default.
#[ic_cdk::update]
fn bare_echo(payload: String) -> usize {
    payload.len()
}

canic::finish!();

thread_local! {
    static RAW_CALLS: Cell<u64> = const { Cell::new(0) };
}

// Framework-only raw protocol: the single command byte is exact and bounded.
// The database integration supplies its own Wire contract downstream.
#[canic::canic_raw_query(public, payload(max_bytes = 16), name = "raw_counts")]
fn native_counts() {
    if ic_cdk::api::msg_arg_data() != [0] {
        ic_cdk::api::msg_reject("invalid raw query");
        return;
    }
    ic_cdk::api::msg_reply(RAW_CALLS.get().to_le_bytes());
}

#[canic::canic_raw_update(
    requires(caller::is_controller()),
    payload(max_bytes = 16),
    name = "raw_commit"
)]
fn native_commit() {
    let bytes = ic_cdk::api::msg_arg_data();
    let action = match bytes.as_slice() {
        [action @ 0..=2] => *action,
        _ => {
            ic_cdk::api::msg_reject("invalid raw command");
            return;
        }
    };
    if action == 1 {
        // An ordinary application failure is a successfully delivered native reply.
        ic_cdk::api::msg_reply([1]);
        return;
    }
    RAW_CALLS.set(RAW_CALLS.get() + 1);
    if action == 2 {
        // Model an adapter that cannot encode its post-handler response.
        ic_cdk::trap("raw reply encoding failed");
    }
    let mut reply = vec![0];
    reply.extend_from_slice(&RAW_CALLS.get().to_le_bytes());
    ic_cdk::api::msg_reply(reply);
}
