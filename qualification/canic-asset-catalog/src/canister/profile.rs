//! Application profile endpoints sharing the catalog's one bootstrap and lifecycle.
use super::{STATE, endpoint_error, lifecycle_guard};
use canic_profile_protocol::{GET, LIMITS, PUT, ProfileError, ProfileResult, Put};
use ic_platform_runtime::{self as runtime, system as api};
use serde::{Serialize, de::DeserializeOwned};

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
    // Reuse the catalog's actual reply-encoding fault boundary for rollback proof.
    let limits = if super::FAILURE.get() == 2 {
        ic_wire::Limits { bytes: 0, ..LIMITS }
    } else {
        LIMITS
    };
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: ID,
            direction: ic_wire::Direction::Response,
        },
        limits,
    )
}

#[runtime::update(runtime = runtime, guard = lifecycle_guard, decode = decode::<Put, PUT>, max_bytes = 4096, on_error = endpoint_error, name = "profile_put", encode = encode::<ProfileResult, PUT>)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoded input"
)]
fn put(input: Put) -> ProfileResult {
    let caller = api::caller();
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("composed application");
        state.user(caller).map_err(|_| ProfileError::Forbidden)?;
        assert!(
            !state.blob.is_drained(),
            "composed application admission closed"
        );
        state.profiles.put(caller.as_slice(), &input)
    })
}

#[runtime::query(runtime = runtime, guard = lifecycle_guard, decode = decode::<u64, GET>, max_bytes = 4096, on_error = endpoint_error, name = "profile_get", encode = encode::<ProfileResult, GET>)]
fn get(user_id: u64) -> ProfileResult {
    let caller = api::caller();
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("composed application");
        state.user(caller).map_err(|_| ProfileError::Forbidden)?;
        assert!(
            !state.blob.is_drained(),
            "composed application admission closed"
        );
        state.profiles.get(caller.as_slice(), user_id)
    })
}
