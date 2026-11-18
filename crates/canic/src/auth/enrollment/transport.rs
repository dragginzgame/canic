//! Fixed enrollment transport; Platform owns execution and Wire owns codecs.
use super::{Error, LIMITS};
use ic_platform_runtime::raw;
use ic_wire::{Direction, Encoding};
use serde::{Serialize, de::DeserializeOwned};

/// Raw header/size admission; the containing authority owns its access guard.
pub fn admit<const ID: u64>() -> Result<(), String> {
    if !(11..=LIMITS.bytes).contains(&raw::msg_arg_data_size()) {
        return Err("enrollment frame size".into());
    }
    let mut header = [0; 11];
    raw::msg_arg_data_copy(&mut header, 0);
    ic_wire::validate_request_header(ID, &header).map_err(|e| e.to_string())
}
/// Decode an admitted complete request; malformed input traps before mutation.
/// # Errors
/// Refuses malformed or over-budget input; the endpoint traps this refusal.
pub fn decode<T: DeserializeOwned, const ID: u64>(bytes: &[u8]) -> Result<T, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        Encoding::Frame {
            method: ID,
            direction: Direction::Request,
        },
        LIMITS,
    )
}
/// Encode in the handler's transaction. Failure traps rather than returning a
/// successful mutation with an ordinary serialization refusal.
/// # Errors
/// Refuses unencodable or over-budget replies; the endpoint traps this refusal.
pub fn encode<T: Serialize, const ID: u64>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        Encoding::Frame {
            method: ID,
            direction: Direction::Response,
        },
        LIMITS,
    )
}
/// Require the current IC controller before beginning any refresh work.
/// # Errors
/// Returns a binding refusal for an unrelated caller, including a former installer.
pub fn authorize_refresh() -> Result<(), Error> {
    use ic_platform_runtime::system;
    if system::is_controller(&system::caller()) {
        Ok(())
    } else {
        Err(Error::Binding)
    }
}

/// Dispatch one fixed, cycle-free refresh request to an approved receiver.
/// # Errors
/// A missing reply leaves the refresh outcome unknown. Never retry implicitly.
pub async fn trigger_refresh(receiver: super::types::Principal) -> Result<(), Error> {
    use ic_platform_runtime::call::{Call, Outcome, Wait};
    let request = super::encode_refresh_request()?;
    let response = Call::new(
        receiver,
        super::REFRESH_METHOD,
        Wait::Bounded { seconds: 30 },
    )
    .args(&request)
    .execute()
    .await
    .map_err(|_| Error::Unavailable)?;
    let Outcome::Reply(bytes) = response.into_parts().0 else {
        return Err(Error::Unavailable);
    };
    super::decode_refresh_response(bytes.as_ref())?
}
