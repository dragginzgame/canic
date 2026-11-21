//! Platform hooks for the fixed Database adapter. Wire alone admits/encodes frames.
use super::RPC_LIMITS;
pub use crate::frame::fail;
pub use ic_platform_runtime::system as api;
use serde::{Serialize, de::DeserializeOwned};

/// Decode the application DTO without adding caller claims.
/// # Errors
/// Refuses malformed or over-budget input; the endpoint traps this refusal.
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: &[u8]) -> Result<T, ic_wire::Error> {
    crate::frame::decode(bytes, ID, RPC_LIMITS)
}

/// Encode after the synchronous handler; failure rolls back all local owners.
/// # Errors
/// Refuses unencodable or over-budget replies; the endpoint traps this refusal.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    crate::frame::encode(&value, ID, RPC_LIMITS)
}

/// Guard before Platform copies the request payload.
/// # Errors
/// Refuses oversize or invalid method/direction framing.
pub fn admit<const ID: u64>() -> Result<(), String> {
    crate::frame::admit(ID, RPC_LIMITS, "Database request size")
}
