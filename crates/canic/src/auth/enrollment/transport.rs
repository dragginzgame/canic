//! Fixed enrollment transport; Platform owns execution and Wire owns codecs.
use super::{Error, LIMITS};
use ic_platform_runtime::{cdk::api, ic0};
use ic_wire::{Direction, Encoding};
use serde::{Serialize, de::DeserializeOwned};

/// Raw header/size admission; the containing authority owns its access guard.
pub fn admit<const ID: u64>() -> Result<(), String> {
    if !(11..=LIMITS.bytes).contains(&ic0::msg_arg_data_size()) {
        return Err("enrollment frame size".into());
    }
    let mut header = [0; 11];
    ic0::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(ID, &header).map_err(|e| e.to_string())
}
/// Decode an admitted complete request; malformed input traps before mutation.
/// # Panics
/// Traps on malformed or over-budget input.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers the admitted input buffer"
)]
#[must_use]
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: Vec<u8>) -> T {
    ic_wire::decode(
        &bytes,
        Encoding::Frame {
            method: ID,
            direction: Direction::Request,
        },
        LIMITS,
    )
    .expect("enrollment request")
}
/// Encode in the handler's transaction. Failure traps rather than returning a
/// successful mutation with an ordinary serialization refusal.
/// # Panics
/// Traps on over-budget or failing serialization.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Vec<u8> {
    ic_wire::encode(
        &value,
        Encoding::Frame {
            method: ID,
            direction: Direction::Response,
        },
        LIMITS,
    )
    .unwrap_or_else(|_| api::trap("enrollment response"))
}
pub(super) fn encode_request(method: u64, value: &impl Serialize) -> Result<Vec<u8>, Error> {
    ic_wire::encode(
        value,
        Encoding::Frame {
            method,
            direction: Direction::Request,
        },
        LIMITS,
    )
    .map_err(|_| Error::Invalid)
}
pub(super) fn decode_response<T: DeserializeOwned>(method: u64, bytes: &[u8]) -> Result<T, Error> {
    ic_wire::decode(
        bytes,
        Encoding::Frame {
            method,
            direction: Direction::Response,
        },
        LIMITS,
    )
    .map_err(|_| Error::Invalid)
}
