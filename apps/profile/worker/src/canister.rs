use crate::state::{MAX_PAGES, State};
use canic_profile_protocol::{GET, INFO, Info, LIMITS, MAX_PROFILES, PUT, ProfileResult, Put};
use ic_platform_runtime::cdk::{self, api};
use ic_storage::DefaultMemoryImpl;
use std::cell::RefCell;

thread_local! { static STATE: RefCell<Option<State<DefaultMemoryImpl>>> = const { RefCell::new(None) }; }

#[cdk::init]
fn init() {
    STATE.with_borrow_mut(|state| *state = Some(State::open(DefaultMemoryImpl::default(), true)));
}

#[cdk::post_upgrade]
fn post_upgrade() {
    STATE.with_borrow_mut(|state| *state = Some(State::open(DefaultMemoryImpl::default(), false)));
    #[cfg(feature = "reject-upgrade")]
    api::trap("profile qualification: reject after restoration");
}

fn bounded() -> Result<(), String> {
    if ic_platform_runtime::ic0::msg_arg_data_size() > LIMITS.bytes {
        return Err("profile frame too large".into());
    }
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
fn decode_put(bytes: Vec<u8>) -> Put {
    ic_wire::native::decode_request(PUT, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid profile request"))
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
fn decode_get(bytes: Vec<u8>) -> u64 {
    ic_wire::native::decode_request(GET, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid profile request"))
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
fn decode_info(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(INFO, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid info request"));
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_put(value: ProfileResult) -> Vec<u8> {
    ic_wire::native::encode_response(PUT, &value, LIMITS).expect("bounded profile")
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_get(value: ProfileResult) -> Vec<u8> {
    ic_wire::native::encode_response(GET, &value, LIMITS).expect("bounded profile")
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_info(value: Info) -> Vec<u8> {
    ic_wire::native::encode_response(INFO, &value, LIMITS).expect("bounded info")
}

#[cdk::update(
    guard = "bounded",
    decode_with = "decode_put",
    encode_with = "encode_put"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoded request"
)]
fn put(input: Put) -> ProfileResult {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("initialized profile state")
            .put(api::msg_caller().as_slice(), &input)
    })
}

#[cdk::query(
    guard = "bounded",
    decode_with = "decode_get",
    encode_with = "encode_get"
)]
fn get(user_id: u64) -> ProfileResult {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("initialized profile state")
            .get(api::msg_caller().as_slice(), user_id)
    })
}

#[cdk::query(
    guard = "bounded",
    decode_with = "decode_info",
    encode_with = "encode_info"
)]
fn info() -> Info {
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
#[cdk::update(
    guard = "bounded",
    decode_with = "decode_info",
    encode_with = "encode_info"
)]
fn probe() -> Info {
    info()
}
