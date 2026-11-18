use crate::{
    state::State,
    storage::{self, MAX_PAGES},
};
use canic_profile_protocol::{GET, INFO, Info, LIMITS, MAX_PROFILES, PUT, ProfileResult, Put};
use ic_platform_runtime::{self as runtime, system as api};
use ic_storage::DefaultMemoryImpl;
use std::cell::RefCell;

thread_local! { static STATE: RefCell<Option<State<DefaultMemoryImpl>>> = const { RefCell::new(None) }; }

#[runtime::init(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn init((): ()) {
    STATE.with_borrow_mut(|state| *state = Some(storage::open(DefaultMemoryImpl::default(), true)));
}

#[runtime::post_upgrade(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn post_upgrade((): ()) {
    STATE
        .with_borrow_mut(|state| *state = Some(storage::open(DefaultMemoryImpl::default(), false)));
    #[cfg(feature = "reject-upgrade")]
    api::trap("profile qualification: reject after restoration");
}

fn bounded() -> Result<(), String> {
    if ic_platform_runtime::system::argument_size() > LIMITS.bytes {
        return Err("profile frame too large".into());
    }
    Ok(())
}

fn decode_put(bytes: &[u8]) -> Result<Put, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: PUT,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
fn decode_get(bytes: &[u8]) -> Result<u64, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: GET,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
fn decode_info(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: INFO,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
#[allow(clippy::needless_pass_by_value)]
fn encode_put(value: ProfileResult) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: PUT,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}
#[allow(clippy::needless_pass_by_value)]
fn encode_get(value: ProfileResult) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: GET,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}
#[allow(clippy::needless_pass_by_value)]
fn encode_info(value: Info) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: INFO,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::update(runtime = runtime, guard = bounded, decode = decode_put, max_bytes = 4096, on_error = endpoint_error, name = "put", encode = encode_put)]
#[allow(clippy::needless_pass_by_value)]
fn put(input: Put) -> ProfileResult {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("initialized profile state")
            .put(api::caller().as_slice(), &input)
    })
}

#[runtime::query(runtime = runtime, guard = bounded, decode = decode_get, max_bytes = 4096, on_error = endpoint_error, name = "get", encode = encode_get)]
fn get(user_id: u64) -> ProfileResult {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("initialized profile state")
            .get(api::caller().as_slice(), user_id)
    })
}

#[runtime::query(runtime = runtime, guard = bounded, decode = decode_info, max_bytes = 4096, on_error = endpoint_error, name = "info", encode = encode_info)]
fn info((): ()) -> Info {
    Info {
        executable: if cfg!(feature = "profile-v2") {
            "profile-v2"
        } else {
            "profile-v1"
        }
        .into(),
        ready: STATE.with_borrow(Option::is_some),
        profile_limit: MAX_PROFILES,
        stable_page_limit: MAX_PAGES,
    }
}

// Replicated, read-only endpoint for the entry's direct inter-canister check.
#[canic::update(contract = canic_profile_protocol::Probe)]
fn probe() -> Info {
    info(())
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
