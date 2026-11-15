//! Public aggregate metrics transport. Platform owns execution and Wire owns framing.
use super::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS, Snapshot};
pub use ic_platform_runtime::cdk;
use ic_platform_runtime::ic0;

/// Validate the bounded request header before Platform copies its payload.
/// # Errors
/// Refuses oversize or invalid method/direction framing.
pub fn admit() -> Result<(), String> {
    let size = ic0::msg_arg_data_size();
    if !(11..=REQUEST_LIMITS.bytes).contains(&size) {
        return Err("metrics request size".into());
    }
    let mut header = [0; 11];
    ic0::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(METHOD, &header).map_err(|e| e.to_string())
}
/// Decode the empty request; callers cannot select labels, authority or providers.
/// # Panics
/// Traps on malformed requests.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers its argument buffer"
)]
pub fn decode(bytes: Vec<u8>) {
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Request,
        },
        REQUEST_LIMITS,
    )
    .expect("metrics request")
}
/// Encode the bounded observation without recording or scheduling work.
/// # Panics
/// Traps if the application exceeds the report contract.
#[must_use]
pub fn encode(value: Snapshot) -> Vec<u8> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Response,
        },
        RESPONSE_LIMITS,
    )
    .expect("metrics reply")
}
