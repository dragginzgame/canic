//! Platform hooks for fixed Auth contracts with directional codec allowances.
use super::issuer;
use super::protocol::{EstablishSession, Logout, ReadSession};
use ic_auth_wire::wire;
pub use ic_platform_runtime as runtime;
use ic_platform_runtime::raw;
pub use ic_platform_runtime::system as api;
use serde::{Serialize, de::DeserializeOwned};
use wire::{Direction, Encoding, Limits};

fn limits<const ID: u64>(direction: Direction) -> Limits {
    match (ID, direction) {
        (EstablishSession::ID, Direction::Request) => EstablishSession::REQUEST_LIMITS,
        (EstablishSession::ID, Direction::Response) => EstablishSession::RESPONSE_LIMITS,
        (ReadSession::ID, Direction::Request) => ReadSession::REQUEST_LIMITS,
        (ReadSession::ID, Direction::Response) => ReadSession::RESPONSE_LIMITS,
        (Logout::ID, Direction::Request) => Logout::REQUEST_LIMITS,
        (Logout::ID, Direction::Response) => Logout::RESPONSE_LIMITS,
        (
            issuer::ISSUE | issuer::RENEW | issuer::RECONCILE | issuer::LOGOUT,
            Direction::Request,
        ) => issuer::REQUEST_LIMITS,
        (
            issuer::ISSUE | issuer::RENEW | issuer::RECONCILE | issuer::LOGOUT,
            Direction::Response,
        ) => issuer::RESPONSE_LIMITS,
        _ => api::trap("unsupported Auth mount identity"),
    }
}

/// Admit raw length and the fixed header before Platform copies the body.
/// # Errors
/// Refuses oversized requests and wrong method/direction/header framing.
pub fn admit<const ID: u64>() -> Result<(), String> {
    if !(11..=limits::<ID>(Direction::Request).bytes).contains(&raw::msg_arg_data_size()) {
        return Err("Auth request size".into());
    }
    let mut header = [0; 11];
    raw::msg_arg_data_copy(&mut header, 0);
    wire::validate_request_header(ID, &header).map_err(|error| error.to_string())
}

/// Decode within the selected complete request budget, without inferring authority.
/// # Errors
/// Refuses malformed or over-budget input; the endpoint traps this refusal.
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: &[u8]) -> Result<T, wire::Error> {
    wire::decode(
        bytes,
        Encoding::Frame {
            method: ID,
            direction: Direction::Request,
        },
        limits::<ID>(Direction::Request),
    )
}

/// Encode in the synchronous handler transaction; failure must roll back all owners.
/// # Errors
/// Refuses unencodable or over-budget replies; the endpoint traps this refusal.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Result<Vec<u8>, wire::Error> {
    wire::encode(
        &value,
        Encoding::Frame {
            method: ID,
            direction: Direction::Response,
        },
        limits::<ID>(Direction::Response),
    )
}

/// Trap admission/encoding failures within the IC message boundary.
/// # Panics
/// Always traps with the refused endpoint stage.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
