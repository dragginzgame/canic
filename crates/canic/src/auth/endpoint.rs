//! Platform hooks for three fixed Auth receiver contracts; Wire owns every codec allowance.
use super::protocol::{EstablishSession, Logout, ReadSession};
pub use ic_platform_runtime::cdk;
pub use ic_platform_runtime::cdk::api;
use ic_platform_runtime::ic0;
use ic_wire::{Direction, Encoding, Limits};
use serde::{Serialize, de::DeserializeOwned};

fn limits<const ID: u64>(direction: Direction) -> Limits {
    match (ID, direction) {
        (EstablishSession::ID, Direction::Request) => EstablishSession::REQUEST_LIMITS,
        (EstablishSession::ID, Direction::Response) => EstablishSession::RESPONSE_LIMITS,
        (ReadSession::ID, Direction::Request) => ReadSession::REQUEST_LIMITS,
        (ReadSession::ID, Direction::Response) => ReadSession::RESPONSE_LIMITS,
        (Logout::ID, Direction::Request) => Logout::REQUEST_LIMITS,
        (Logout::ID, Direction::Response) => Logout::RESPONSE_LIMITS,
        _ => api::trap("unsupported Auth mount identity"),
    }
}

/// Admit raw length and the fixed header before Platform copies the body.
/// # Errors
/// Refuses oversized requests and wrong method/direction/header framing.
pub fn admit<const ID: u64>() -> Result<(), String> {
    if !(11..=limits::<ID>(Direction::Request).bytes).contains(&ic0::msg_arg_data_size()) {
        return Err("Auth request size".into());
    }
    let mut header = [0; 11];
    ic0::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(ID, &header).map_err(|error| error.to_string())
}

/// Decode within the supplier's complete request budget, without inferring authority.
/// # Panics
/// Traps on malformed input or a resource-bound violation.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform transfers its admitted argument buffer"
)]
#[must_use]
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: Vec<u8>) -> T {
    ic_wire::decode(
        &bytes,
        Encoding::Frame {
            method: ID,
            direction: Direction::Request,
        },
        limits::<ID>(Direction::Request),
    )
    .expect("Auth request")
}

/// Encode in the synchronous handler transaction; failure must roll back all owners.
/// # Panics
/// Traps on encoding failure instead of returning an ordinary post-mutation error.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Vec<u8> {
    ic_wire::encode(
        &value,
        Encoding::Frame {
            method: ID,
            direction: Direction::Response,
        },
        limits::<ID>(Direction::Response),
    )
    .expect("Auth reply")
}
