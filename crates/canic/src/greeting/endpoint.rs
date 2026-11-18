//! Fixed demo adapter. Applications own the handler; Platform and Wire own execution and framing.
use crate::{GREET, LIMITS, MAX_NAME_BYTES};
pub use ic_platform_runtime as runtime;
use ic_platform_runtime::system as api;

/// Bound the greeting frame before Platform copies the request.
/// # Errors
/// Refuses oversized requests without invoking the application.
pub fn bounded() -> Result<(), String> {
    if api::argument_size() > LIMITS.bytes {
        return Err("greeting frame too large".into());
    }
    Ok(())
}

/// Decode the existing greeting contract and its bounded UTF-8 name.
/// # Errors
/// The return type is infallible; invalid requests trap.
/// # Panics
/// Traps on invalid Wire framing or a name exceeding the application limit.
#[must_use]
pub fn decode(bytes: &[u8]) -> Result<String, std::convert::Infallible> {
    let name: String = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: GREET,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap_or_else(|_| api::trap("invalid greeting frame"));
    if name.len() > MAX_NAME_BYTES {
        api::trap("name exceeds 128 UTF-8 bytes");
    }
    Ok(name)
}

/// Trap refused endpoint stages within the current message.
/// # Panics
/// Always traps.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    api::trap(&format!("endpoint refused: {error:?}"))
}
