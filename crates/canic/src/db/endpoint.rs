//! Platform hooks for the fixed Database adapter. Wire alone admits/encodes frames.
use super::RPC_LIMITS;
pub use ic_platform_runtime as runtime;
use ic_platform_runtime::raw;
pub use ic_platform_runtime::system as api;
use serde::{Serialize, de::DeserializeOwned};

/// Guard before Platform copies the request payload.
/// # Errors
/// Refuses oversize or invalid method/direction framing.
pub fn admit<const ID: u64>() -> Result<(), String> {
    let size = raw::msg_arg_data_size();
    if !(11..=RPC_LIMITS.bytes).contains(&size) {
        return Err("Database request size".into());
    }
    let mut header = [0; 11];
    raw::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(ID, &header).map_err(|error| error.to_string())
}
/// Decode the application DTO without adding caller claims.
/// # Errors
/// Refuses malformed or over-budget input; the endpoint traps this refusal.
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: &[u8]) -> Result<T, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: ID,
            direction: ic_wire::Direction::Request,
        },
        RPC_LIMITS,
    )
}
/// Encode after the synchronous handler; failure rolls back all local owners.
/// # Errors
/// Refuses unencodable or over-budget replies; the endpoint traps this refusal.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: ID,
            direction: ic_wire::Direction::Response,
        },
        RPC_LIMITS,
    )
}

/// Trap admission/encoding failures within the IC message boundary.
/// # Panics
/// Always traps with the refused endpoint stage.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
