//! Fixed demo adapter. Applications own the handler; Platform and Wire own execution and framing.
use crate::{GREET, Greeting, LIMITS, MAX_NAME_BYTES};
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

/// Encode the fixed response; Platform traps any encoding failure.
/// # Errors
/// Refuses an unencodable or over-budget response.
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint encoder owns return value"
)]
pub fn encode(value: Greeting) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: GREET,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

/// Trap refused endpoint stages within the current message.
/// # Panics
/// Always traps.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    api::trap(&format!("endpoint refused: {error:?}"))
}

/// Mount the demo's public `greet` query and replicated `probe` update.
/// The explicit synchronous handler accepts `&str` and returns `canic::Greeting`.
/// Mounting starts no storage, timers or lifecycle and grants no authority.
/// This fixed greeting probe does not establish readiness for other applications.
#[macro_export]
macro_rules! mount_greeting_endpoints {
    ($handler:path) => {
        mod canic_greeting {
            use $crate::{Greeting, greeting as endpoint};
            use endpoint::runtime;

            #[runtime::query(runtime = runtime, guard = endpoint::bounded, decode = endpoint::decode, max_bytes = 4096, on_error = endpoint::fail, name = "greet", encode = endpoint::encode)]
            #[expect(clippy::needless_pass_by_value, reason = "entrypoint owns the decoded request")]
            fn greet(name: String) -> Greeting {
                $handler(&name)
            }

            #[runtime::update(runtime = runtime, guard = endpoint::bounded, decode = endpoint::decode, max_bytes = 4096, on_error = endpoint::fail, name = "probe", encode = endpoint::encode)]
            #[expect(clippy::needless_pass_by_value, reason = "entrypoint owns the decoded request")]
            fn probe(name: String) -> Greeting {
                $handler(&name)
            }
        }
    };
}
