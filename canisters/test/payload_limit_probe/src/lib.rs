#![expect(clippy::unused_async)]
mod codec;

use canic::access::{AccessCheck, AccessContext, AccessError};
use canic::endpoint::ArgumentLimits;
use canic::{Error, candid_query, candid_update, prelude::*};
use ic_platform::cdk::call::Call;
use ic_platform::{candid::contract::CandidType, principal::ExternalPrincipal as Principal};
use std::cell::Cell;

const LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 10_000,
    skipping_quota: 32,
    max_type_len: 8,
    max_header_len: 128,
};
const LIFECYCLE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 65_536,
    nodes: 4096,
    depth: 32,
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
    ic_platform::cdk::api::debug_print("bounded participant init");
    // Prove that participant witnesses survive a failed install, even though
    // reinstall clears the previous canister logs before entering init.
    let (args,): (Option<Vec<u8>>,) = canic::endpoint::read_initialization(LIFECYCLE_LIMITS)
        .expect("the framework already admitted this envelope");
    if args.as_deref() == Some(&[1]) {
        ic_platform::cdk::api::debug_print("bounded participant trap witness");
        ic_platform::cdk::api::trap("participant witness");
    }
}
fn bounded_post_upgrade() {
    ic_platform::cdk::api::debug_print("bounded participant post_upgrade");
}

/// A plain protocol record, with no Result envelope on the wire.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
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
    ic_platform::cdk::api::debug_print("bounded handler dispatched");
    HANDLER_CALLS.set(HANDLER_CALLS.get() + 1);
    PlainReply {
        committed: HANDLER_CALLS.get(),
        predicates: PREDICATE_CALLS.get(),
    }
}

#[candid_update(requires(custom(Gate(allow)), custom(Gate(true))), on_access_denied = "reject", decode = LIMITS)]
fn plain_commit(allow: bool) -> PlainReply {
    assert!(allow);
    commit()
}

#[candid_update(requires(custom(Gate(allow)), custom(Gate(true))), on_access_denied = "reject", decode = LIMITS)]
fn result_commit(allow: bool) -> Result<PlainReply, String> {
    assert!(allow);
    Ok(commit())
}

#[candid_query(public, on_access_denied = "reject", decode = LIMITS)]
fn dispatch_counts() -> PlainReply {
    PlainReply {
        committed: HANDLER_CALLS.get(),
        predicates: PREDICATE_CALLS.get(),
    }
}

// This probe reports one observed security counter without exporting the internal metrics view.
#[query(public, on_access_denied = "reject", wire(id = 7010, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn access_counts(method: String) -> u128 {
    canic::api::metrics::MetricsQuery::security(canic::dto::page::PageRequest {
        limit: 100,
        offset: 0,
    })
    .entries
    .into_iter()
    .filter(|entry| {
        entry.labels.first().is_some_and(|label| label == "access")
            && entry.labels.get(1).is_some_and(|label| label == &method)
    })
    .map(|entry| match entry.value {
        canic::dto::metrics::MetricValue::Count { count, exact: true } => count,
        _ => panic!("security metrics must count denials"),
    })
    .sum()
}

#[candid_query(public, on_access_denied = "reject", decode = LIMITS)]
fn bounded_query(value: String) -> PlainReply {
    drop(value);
    commit()
}

#[candid_update(public, on_access_denied = "reject", decode = LIMITS)]
fn bounded_update(value: String) -> PlainReply {
    drop(value);
    commit()
}

#[candid_update(public, on_access_denied = "reject", decode = WORK_LIMITS)]
fn bounded_work(value: Vec<()>) -> PlainReply {
    drop(value);
    commit()
}

#[candid_update(public, on_access_denied = "reject", decode = LIMITS)]
fn bounded_skip() -> PlainReply {
    commit()
}

#[candid_update(public, on_access_denied = "reject", decode = TYPE_LIMITS)]
fn bounded_types(value: Vec<u8>) -> PlainReply {
    drop(value);
    commit()
}

#[candid_update(public, on_access_denied = "reject", decode = HEADER_LIMITS)]
fn bounded_header(value: u64) -> PlainReply {
    let _ = value;
    commit()
}

/// Relay arbitrary bytes so update refusal is exercised without inspect_message.
#[candid_update(on_access_denied = "reject", public, payload(max_bytes = 16 * 1024))]
async fn relay_bounded(target: Principal, method: String, bytes: Vec<u8>) -> Result<bool, String> {
    Call::unbounded_wait(target.into(), &method)
        .take_raw_args(bytes)
        .await
        .map(|_| true)
        .map_err(|error| error.to_string())
}

// Provide an empty setup hook so `start!` can schedule user lifecycle work.
async fn canic_setup() {}

// Provide an empty install hook; payload tests only exercise update ingress.
async fn canic_install(_: Option<Vec<u8>>) {}

// Provide an empty upgrade hook for the required Canic lifecycle surface.
async fn canic_upgrade() {}

/// Relay a generated payload through a real inter-canister update.
#[candid_update(on_access_denied = "reject", public)]
async fn relay_explicit_echo(target: Principal, len: usize) -> Result<usize, String> {
    let bytes = ic_platform::candid::contract::encode_one("x".repeat(len))
        .map_err(|error| error.to_string())?;
    let response = Call::unbounded_wait(target.into(), "explicit_echo")
        .take_raw_args(bytes)
        .await
        .map_err(|error| error.to_string())?;
    let (result,): (Result<usize, String>,) =
        ic_platform::candid::contract::decode_args_with_config(
            &response.into_bytes(),
            &ic_platform::candid::contract::DecoderConfig::new(),
        )
        .map_err(|error| error.to_string())?;
    result
}

/// Echo payload length under the default update ingress limit.
#[candid_update(on_access_denied = "reject", public)]
fn default_echo(payload: String) -> Result<usize, String> {
    Ok(payload.len())
}

/// Echo payload length under an explicit larger update ingress limit.
#[candid_update(on_access_denied = "reject", public, payload(max_bytes = 32 * 1024))]
fn explicit_echo(payload: String) -> Result<usize, String> {
    Ok(payload.len())
}

/// Echo payload length under an explicit limit and exported method name.
#[candid_update(on_access_denied = "reject", public, name = "wire_named_echo", payload(max_bytes = 24 * 1024))]
fn named_echo(payload: String) -> Result<usize, String> {
    Ok(payload.len())
}

/// Bare CDK updates still inherit the managed ingress inspector default.
#[cfg_attr(
    any(not(target_arch = "wasm32"), canic_export_candid),
    ic_platform::candid::contract::candid_method(update)
)]
#[ic_platform::cdk::update(decode_with = "codec::decode_one", encode_with = "codec::encode_reply")]
fn bare_echo(payload: String) -> usize {
    payload.len()
}

const NATIVE_PRINCIPAL_METHOD_ID: u64 = 7001;
const NATIVE_COMMIT_METHOD_ID: u64 = 7002;

const WIRE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 4096,
    nodes: 64,
    depth: 8,
};

/// Exercise the native identity without a Candid implementation or codec.
#[query(public, on_access_denied = "reject", wire(id = crate::NATIVE_PRINCIPAL_METHOD_ID, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn native_principal(principal: ic_platform::Principal) -> ic_platform::Principal {
    principal
}

/// A synchronous mutation must roll back if its Wire reply exceeds the allowance.
#[update(requires(custom(Gate(length > 0))), on_access_denied = "reject", wire(id = crate::NATIVE_COMMIT_METHOD_ID, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn native_commit(length: u32) -> Vec<u8> {
    let _ = commit();
    vec![0; length as usize]
}

const NATIVE_RAW_METHOD_ID: u64 = 7003;

// The codec owner supplies its actual projection; a raw unit signature is not a schema.
canic::__canic_register_wire_schema!(
    "native_raw",
    Update,
    NATIVE_RAW_METHOD_ID,
    u32,
    Vec<u8>,
    WIRE_LIMITS,
    WIRE_LIMITS
);

#[canic::raw_update(public, wire(id = crate::NATIVE_RAW_METHOD_ID, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn native_raw() {
    ic_platform::cdk::api::debug_print("raw Wire owner entered");
    let length: u32 = ic_wire::native::decode_request(
        NATIVE_RAW_METHOD_ID,
        &ic_platform::cdk::api::msg_arg_data(),
        WIRE_LIMITS,
    )
    .unwrap();
    let _ = commit();
    let bytes = ic_wire::native::encode_response(
        NATIVE_RAW_METHOD_ID,
        &vec![0_u8; length as usize],
        WIRE_LIMITS,
    )
    .unwrap();
    ic_platform::cdk::api::msg_reply(bytes);
}

const NATIVE_COMPOSITE_METHOD_ID: u64 = 7004;

/// Qualify async Wire reply encoding after a real composite-query callback.
#[query(public, composite, on_access_denied = "reject", wire(id = crate::NATIVE_COMPOSITE_METHOD_ID, request = WIRE_LIMITS, response = WIRE_LIMITS))]
async fn native_composite(target: ic_platform::Principal) -> ic_platform::Principal {
    let own = ic_platform::cdk::api::canister_self();
    let request =
        ic_wire::native::encode_request(NATIVE_PRINCIPAL_METHOD_ID, &own, WIRE_LIMITS).unwrap();
    let response = ic_platform::cdk::call::Call::unbounded_wait(target, "native_principal")
        .with_raw_args(&request)
        .await
        .unwrap();
    ic_wire::native::decode_response(
        NATIVE_PRINCIPAL_METHOD_ID,
        &response.into_bytes(),
        WIRE_LIMITS,
    )
    .unwrap()
}

const NATIVE_RESULT_METHOD_ID: u64 = 7005;

/// Preserve typed access denials without dispatching the application handler.
#[update(requires(custom(Gate(allow)), custom(Gate(true))), wire(id = crate::NATIVE_RESULT_METHOD_ID, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn native_result(allow: bool) -> Result<u64, Error> {
    assert!(allow);
    Ok(commit().committed)
}

canic::finish!();

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
