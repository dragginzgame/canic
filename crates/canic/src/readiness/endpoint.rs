//! Platform executes the message; Wire admits frames; the application supplies its state.
use super::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS, State};

#[cfg(target_arch = "wasm32")]
pub use ingress::{admit, fail, runtime};

/// Decode an empty observation request. No request field selects or activates state.
/// # Errors
/// Refuses malformed or over-budget requests; Platform traps the refusal.
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

/// Encode the application's observation without changing its lifecycle.
/// # Errors
/// Refuses an unencodable response; Platform traps within the same message.
pub fn encode(state: State) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &state,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Response,
        },
        RESPONSE_LIMITS,
    )
}

#[cfg(target_arch = "wasm32")]
mod ingress {
    use super::{METHOD, REQUEST_LIMITS};
    pub use ic_platform_runtime as runtime;
    use ic_platform_runtime::raw;

    /// Admit the small request header before Platform copies the full argument.
    /// # Errors
    /// Refuses invalid size, method or direction.
    pub fn admit() -> Result<(), String> {
        let size = raw::msg_arg_data_size();
        if !(11..=REQUEST_LIMITS.bytes).contains(&size) {
            return Err("readiness request size".into());
        }
        let mut header = [0; 11];
        raw::msg_arg_data_copy(&mut header, 0);
        ic_wire::validate_request_header(METHOD, &header).map_err(|error| error.to_string())
    }

    /// Trap refused endpoint stages within the current message.
    /// # Panics
    /// Always traps.
    pub fn fail(error: impl std::fmt::Debug) -> ! {
        runtime::system::trap(&format!("endpoint refused: {error:?}"))
    }
}
