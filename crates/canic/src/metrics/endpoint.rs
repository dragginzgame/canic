//! Public aggregate metrics transport. Platform owns execution and Wire owns framing.
use super::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS, Snapshot};
pub use ic_platform_runtime as runtime;
use ic_platform_runtime::raw;

/// Decode the empty request; callers cannot select labels, authority or providers.
/// # Errors
/// Refuses malformed or over-budget input; the endpoint traps this refusal.
pub fn decode(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Request,
        },
        REQUEST_LIMITS,
    )
}

/// Encode the bounded observation without recording or scheduling work.
/// # Errors
/// Refuses a report exceeding its contract; the endpoint traps this refusal.
pub fn encode(value: Snapshot) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Response,
        },
        RESPONSE_LIMITS,
    )
}

/// Validate the bounded request header before Platform copies its payload.
/// # Errors
/// Refuses oversize or invalid method/direction framing.
pub fn admit() -> Result<(), String> {
    let size = raw::msg_arg_data_size();
    if !(11..=REQUEST_LIMITS.bytes).contains(&size) {
        return Err("metrics request size".into());
    }
    let mut header = [0; 11];
    raw::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(METHOD, &header).map_err(|e| e.to_string())
}

/// Trap admission/encoding failures within the IC message boundary.
/// # Panics
/// Always traps with the refused endpoint stage.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
