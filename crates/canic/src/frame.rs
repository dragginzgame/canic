//! Framing for synchronous Blob and DB mounts.
use ic_platform_runtime::raw;
use ic_wire::{Direction, Encoding, Error, Limits};
use serde::{Serialize, de::DeserializeOwned};

pub(crate) fn admit(method: u64, limits: Limits, size_error: &str) -> Result<(), String> {
    let mut header = [0; 11];
    if !(header.len()..=limits.bytes).contains(&raw::msg_arg_data_size()) {
        return Err(size_error.into());
    }
    raw::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(method, &header).map_err(|error| error.to_string())
}

pub(crate) fn decode<T: DeserializeOwned>(
    bytes: &[u8],
    method: u64,
    limits: Limits,
) -> Result<T, Error> {
    ic_wire::decode(
        bytes,
        Encoding::Frame {
            method,
            direction: Direction::Request,
        },
        limits,
    )
}

pub(crate) fn encode<T: Serialize>(
    value: &T,
    method: u64,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    ic_wire::encode(
        value,
        Encoding::Frame {
            method,
            direction: Direction::Response,
        },
        limits,
    )
}

/// Trap admission/encoding failures within the IC message boundary.
/// # Panics
/// Always traps with the refused endpoint stage.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
