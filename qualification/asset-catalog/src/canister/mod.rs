//! Explicit application lifecycle and local transactions; no self calls or timers.
mod state;
use crate::protocol::{
    self, AUTHORIZE, Asset, Draft, Entry, Error, FAULT, INIT, INSPECT, PUBLISH, REMOVE, UPGRADE,
};
use canic::blob::{
    endpoint,
    supplier::{
        local::Error as BlobError,
        protocol::{Command, Inspection, ManagementView, Observation, Outcome, RPC_LIMITS},
    },
};
use ic_platform_runtime::cdk::{self, api};
use ic_platform_types::Principal;
use state::State;
use std::cell::{Cell, RefCell};

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    // This package is a qualification application; the product adapter has no fault mode.
    static FAILURE: Cell<u8> = const { Cell::new(0) };
}
#[cdk::init(decode_with = "endpoint::decode::<protocol::Installation, INIT>")]
fn init(input: protocol::Installation) {
    STATE.with_borrow_mut(|state| *state = Some(State::open(Some(input), api::canister_self())));
}
#[cdk::pre_upgrade]
fn pre_upgrade() {
    STATE.with_borrow(|state| {
        assert!(
            state.as_ref().expect("catalog").blob.is_drained(),
            "drain Blob before upgrade"
        )
    });
}
#[cdk::post_upgrade(decode_with = "endpoint::decode::<(), UPGRADE>")]
fn post_upgrade() {
    STATE.with_borrow_mut(|state| *state = Some(State::open(None, api::canister_self())));
}
canic::mount_blob_endpoints! {
    execute = crate::canister::execute_blob,
    inspect = crate::canister::inspect_blob,
    management = crate::canister::inspect_management,
}
fn execute_blob(caller: Principal, command: Command, now: u64) -> Result<Outcome, BlobError> {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("catalog")
            .blob
            .execute(caller, command, now)
    })
}
fn inspect_blob(caller: Principal, input: Inspection) -> Result<Observation, BlobError> {
    STATE.with_borrow(|state| state.as_ref().expect("catalog").blob.inspect(caller, input))
}
fn inspect_management(caller: Principal) -> Result<ManagementView, BlobError> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("catalog")
            .blob
            .inspect_management(caller)
    })
}
#[cdk::update(
    guard = "endpoint::admit::<AUTHORIZE>",
    decode_with = "endpoint::decode::<Draft, AUTHORIZE>",
    encode_with = "encode::<Result<Entry, Error>, AUTHORIZE>"
)]
fn catalog_authorize(input: Draft) -> Result<Entry, Error> {
    STATE.with_borrow_mut(|state| {
        state.as_mut().expect("catalog").authorize(
            api::msg_caller(),
            input,
            api::time(),
            FAILURE.get(),
        )
    })
}
#[cdk::update(
    guard = "endpoint::admit::<PUBLISH>",
    decode_with = "endpoint::decode::<std::num::NonZeroU128, PUBLISH>",
    encode_with = "encode::<Result<Entry, Error>, PUBLISH>"
)]
fn catalog_publish(reference: std::num::NonZeroU128) -> Result<Entry, Error> {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("catalog")
            .publish(api::msg_caller(), reference, FAILURE.get())
    })
}
#[cdk::update(
    guard = "endpoint::admit::<REMOVE>",
    decode_with = "endpoint::decode::<std::num::NonZeroU128, REMOVE>",
    encode_with = "encode::<Result<Entry, Error>, REMOVE>"
)]
fn catalog_remove(reference: std::num::NonZeroU128) -> Result<Entry, Error> {
    STATE.with_borrow_mut(|state| {
        state.as_mut().expect("catalog").remove(
            api::msg_caller(),
            reference,
            api::time(),
            FAILURE.get(),
        )
    })
}
#[cdk::query(
    guard = "endpoint::admit::<INSPECT>",
    decode_with = "endpoint::decode::<std::num::NonZeroU128, INSPECT>",
    encode_with = "endpoint::encode::<Result<Asset, Error>, INSPECT>"
)]
fn catalog_inspect(reference: std::num::NonZeroU128) -> Result<Asset, Error> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("catalog")
            .inspect(api::msg_caller(), reference)
    })
}
type Ack = ();
#[cdk::update(
    guard = "endpoint::admit::<FAULT>",
    decode_with = "endpoint::decode::<u8, FAULT>",
    encode_with = "endpoint::encode::<(), FAULT>"
)]
fn catalog_fault(point: u8) -> Ack {
    assert!(api::is_controller(&api::msg_caller()));
    assert!(point <= 2);
    FAILURE.set(point);
}
// Use the same greeting/readiness contract as Canic's current Root. Inspection
// and code identity cannot substitute for the explicit Blob activation below.
const GREET: u64 = canic::GREET;
#[cdk::query(
    guard = "endpoint::admit::<GREET>",
    decode_with = "endpoint::decode::<String, GREET>",
    encode_with = "endpoint::encode::<canic::Greeting, GREET>"
)]
fn probe(_name: String) -> canic::Greeting {
    STATE.with_borrow(|state| {
        assert!(
            !state.as_ref().expect("catalog").blob.is_drained(),
            "Blob admission remains closed"
        )
    });
    canic::Greeting {
        message: if cfg!(feature = "maintenance-candidate") {
            "Catalog candidate"
        } else {
            "Catalog original"
        }
        .into(),
    }
}
fn encode<T: serde::Serialize, const ID: u64>(value: T) -> Vec<u8> {
    let limits = if FAILURE.get() == 2 {
        ic_wire::Limits {
            bytes: 0,
            ..RPC_LIMITS
        }
    } else {
        RPC_LIMITS
    };
    ic_wire::native::encode_response(ID, &value, limits).expect("catalog reply encoding")
}
