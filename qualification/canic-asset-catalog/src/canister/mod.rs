//! Explicit application lifecycle and local transactions; no self calls or timers.
use canic::blob::RPC_LIMITS;
#[cfg(feature = "profile")]
mod profile;
mod state;
use crate::protocol::{
    self, AUTHORIZE, Asset, Draft, Entry, Error, FAULT, INIT, INSPECT, PUBLISH, REMOVE, UPGRADE,
};
use canic::blob::{
    endpoint,
    supplier::{
        local::Error as BlobError,
        protocol::{Command, Inspection, ManagementView, Observation, Outcome},
    },
};
use ic_platform_runtime::{self as runtime, system as api};
use ic_platform_types::Principal;
use state::State;
use std::cell::{Cell, RefCell};

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    // This package is a qualification application; the product adapter has no fault mode.
    static FAILURE: Cell<u8> = const { Cell::new(0) };
}

/// Initialize the asset catalog state for this installation.
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = endpoint::decode::<protocol::Installation, INIT>,
    max_bytes = 2097152,
    on_error = endpoint_error,
)]
fn init(input: protocol::Installation) {
    STATE.with_borrow_mut(|state| *state = Some(State::open(Some(input), api::canister_self())));
}

/// Refuse upgrade until the embedded Blob service is drained.
#[runtime::pre_upgrade(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn pre_upgrade((): ()) {
    STATE.with_borrow(|state| {
        assert!(
            state.as_ref().expect("catalog").blob.is_drained(),
            "drain Blob before upgrading catalog {}",
            if cfg!(feature = "maintenance-candidate") {
                "candidate"
            } else {
                "original"
            }
        )
    });
}

/// Reopen the asset catalog state from the preceding executable.
#[runtime::post_upgrade(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = endpoint::decode::<(), UPGRADE>,
    max_bytes = 2097152,
    on_error = endpoint_error,
)]
fn post_upgrade((): ()) {
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

/// Authorize the caller's draft under the catalog policy.
#[runtime::update(
    runtime = runtime,
    guard = endpoint::admit::<AUTHORIZE>,
    decode = endpoint::decode::<Draft, AUTHORIZE>,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "catalog_authorize",
    encode = encode::<Result<Entry, Error>, AUTHORIZE>,
)]
fn catalog_authorize(input: Draft) -> Result<Entry, Error> {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("catalog")
            .authorize(api::caller(), input, api::time(), FAILURE.get())
    })
}

/// Publish the selected catalog reference under its owner's policy.
#[runtime::update(
    runtime = runtime,
    guard = endpoint::admit::<PUBLISH>,
    decode = endpoint::decode::<std::num::NonZeroU128, PUBLISH>,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "catalog_publish",
    encode = encode::<Result<Entry, Error>, PUBLISH>,
)]
fn catalog_publish(reference: std::num::NonZeroU128) -> Result<Entry, Error> {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("catalog")
            .publish(api::caller(), reference, FAILURE.get())
    })
}

/// Release the selected catalog use.
#[runtime::update(
    runtime = runtime,
    guard = endpoint::admit::<REMOVE>,
    decode = endpoint::decode::<std::num::NonZeroU128, REMOVE>,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "catalog_remove",
    encode = encode::<Result<Entry, Error>, REMOVE>,
)]
fn catalog_remove(reference: std::num::NonZeroU128) -> Result<Entry, Error> {
    STATE.with_borrow_mut(|state| {
        state.as_mut().expect("catalog").remove(
            api::caller(),
            reference,
            api::time(),
            FAILURE.get(),
        )
    })
}

/// Inspect the asset through its catalog reference.
#[runtime::query(
    runtime = runtime,
    guard = endpoint::admit::<INSPECT>,
    decode = endpoint::decode::<std::num::NonZeroU128, INSPECT>,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "catalog_inspect",
    encode = endpoint::encode::<Result<Asset, Error>, INSPECT>,
)]
fn catalog_inspect(reference: std::num::NonZeroU128) -> Result<Asset, Error> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("catalog")
            .inspect(api::caller(), reference)
    })
}
type Ack = ();

/// Select the controller's failure point for catalog rollback qualification.
#[runtime::update(
    runtime = runtime,
    guard = endpoint::admit::<FAULT>,
    decode = endpoint::decode::<u8, FAULT>,
    max_bytes = 2097152,
    on_error = endpoint_error,
    name = "catalog_fault",
    encode = endpoint::encode::<(), FAULT>,
)]
fn catalog_fault(point: u8) -> Ack {
    assert!(api::is_controller(&api::caller()));
    assert!(point <= 2);
    FAILURE.set(point);
}

canic::mount_readiness_endpoint!(crate::canister::readiness);

fn readiness() -> canic::readiness::State {
    use canic::readiness::State::{Fenced, Ready, Unavailable};
    STATE.with_borrow(|state| match state {
        None => Unavailable,
        Some(state) if state.blob.is_drained() => Fenced,
        Some(_) => Ready,
    })
}

fn encode<T: serde::Serialize, const ID: u64>(
    value: T,
) -> Result<Vec<u8>, std::convert::Infallible> {
    Ok({
        let limits = if FAILURE.get() == 2 {
            ic_wire::Limits {
                bytes: 0,
                ..RPC_LIMITS
            }
        } else {
            RPC_LIMITS
        };
        ic_wire::encode(
            &value,
            ic_wire::Encoding::Frame {
                method: ID,
                direction: ic_wire::Direction::Response,
            },
            limits,
        )
        .expect("catalog reply encoding")
    })
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

/// The IC admits lifecycle calls. The declared argument budget still applies.
fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
