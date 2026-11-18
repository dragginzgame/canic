//! Real profile persistence under the fixture's single protected issuer namespace.
//! The optional witness composes the existing application engine, not a copy of it.
use canic_profile_protocol::{GET, LIMITS, PUT, ProfileError, ProfileResult, Put};
use canic_profile_worker::state::State;
use ic_platform_runtime::{self as runtime, system as api};
use ic_storage::DefaultMemoryImpl;
use serde::{Serialize, de::DeserializeOwned};
use std::cell::RefCell;

thread_local! { static PROFILES: RefCell<Option<State<DefaultMemoryImpl>>> = const { RefCell::new(None) }; }
thread_local! { static WRITES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }

pub(crate) fn open(fresh: bool) {
    PROFILES.with_borrow_mut(|state| {
        *state = Some(canic_profile_worker::storage::open(
            DefaultMemoryImpl::default(),
            fresh,
        ))
    });
}

fn guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}
fn empty(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err("no upgrade arguments".into())
    }
}
fn fail(error: impl std::fmt::Debug) -> ! {
    api::trap(&format!("profile endpoint refused: {error:?}"))
}

#[runtime::post_upgrade(runtime = runtime, guard = guard, decode = empty, max_bytes = 0, on_error = fail)]
fn reopen((): ()) {
    open(false);
    // Auth state deliberately stays absent: persistent profile bytes cannot
    // reconstruct lost login/replay authority. No fresh enrollment workaround.
}
fn decode<T: DeserializeOwned, const ID: u64>(bytes: &[u8]) -> Result<T, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: ID,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
fn encode<T: Serialize, const ID: u64>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    let frame = ic_wire::Encoding::Frame {
        method: ID,
        direction: ic_wire::Direction::Response,
    };
    if crate::canister::profile_fault() {
        ic_wire::encode(&crate::canister::Unencodable, frame, LIMITS)
    } else {
        ic_wire::encode(&value, frame, LIMITS)
    }
}
fn owner(scope: &str) -> Result<ic_auth_protocol_types::Principal, ProfileError> {
    let receipt = crate::canister::read_scope(api::caller(), api::time(), scope)
        .map_err(|_| ProfileError::Unauthenticated)?;
    // read_scope validates the issuer against the sole protected Root grant.
    // No caller-supplied issuer/subject or controller membership is consulted.
    Ok(receipt.subject)
}

#[runtime::update(runtime = runtime, guard = guard, decode = decode::<Put, PUT>, max_bytes = 4096, on_error = fail, name = "profile_put", encode = encode::<ProfileResult, PUT>)]
#[allow(clippy::needless_pass_by_value)]
fn put(input: Put) -> ProfileResult {
    let owner = owner("write")?;
    let result = PROFILES.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("profile store open")
            .put(owner.as_slice(), &input)
    });
    if result.is_ok() {
        WRITES.set(WRITES.get() + 1);
    }
    result
}

#[runtime::query(runtime = runtime, guard = guard, decode = crate::canister::decode_observe, max_bytes = 0, on_error = fail, name = "observe_profile_writes", encode = crate::canister::encode_observe::<u32>)]
fn writes((): ()) -> u32 {
    WRITES.get()
}

// Replicated read also exercises the same authenticated HTTP update transport.
#[runtime::update(runtime = runtime, guard = guard, decode = decode::<u64, GET>, max_bytes = 4096, on_error = fail, name = "profile_get", encode = encode::<ProfileResult, GET>)]
fn get(user_id: u64) -> ProfileResult {
    let owner = owner("read")?;
    PROFILES.with_borrow(|state| {
        state
            .as_ref()
            .expect("profile store open")
            .get(owner.as_slice(), user_id)
    })
}
