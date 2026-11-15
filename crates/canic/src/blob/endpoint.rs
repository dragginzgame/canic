//! Platform hooks for the fixed Blob adapter. Wire alone admits/encodes frames.
use super::RPC_LIMITS;
pub use ic_platform_runtime::cdk;
pub use ic_platform_runtime::cdk::api;
use ic_platform_runtime::ic0;
use serde::{Serialize, de::DeserializeOwned};

/// Guard before Platform copies the request payload.
/// # Errors
/// Refuses oversize or invalid method/direction framing.
pub fn admit<const ID: u64>() -> Result<(), String> {
    let size = ic0::msg_arg_data_size();
    if size > RPC_LIMITS.bytes || size < 11 {
        return Err("Blob request size".into());
    }
    let mut header = [0; 11];
    ic0::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(ID, &header).map_err(|error| error.to_string())
}
/// Decode the supplier DTO without adding caller claims.
/// # Panics
/// Traps on malformed or over-budget payloads.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers its argument buffer"
)]
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: Vec<u8>) -> T {
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: ID,
            direction: ic_wire::Direction::Request,
        },
        RPC_LIMITS,
    )
    .expect("Blob request")
}
/// Encode after the synchronous handler; failure rolls back all local owners.
/// # Panics
/// Traps on reply encoding failure; never substitutes an ordinary error reply.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Vec<u8> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: ID,
            direction: ic_wire::Direction::Response,
        },
        RPC_LIMITS,
    )
    .expect("Blob reply")
}
