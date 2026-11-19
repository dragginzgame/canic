//! Named application Wire endpoints with Platform execution.
mod actor;

pub use actor::{CallError, admit_request, call};

#[doc(hidden)]
pub use ic_platform_runtime as runtime;
#[doc(hidden)]
pub use ic_wire as wire;

/// Abort the current endpoint execution on a refused stage.
/// # Panics
/// Traps the current IC message or panics when executing a native handler.
#[doc(hidden)]
pub fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

/// Default Wire budgets for small application requests and replies.
pub const LIMITS: wire::Limits = wire::Limits {
    bytes: 512,
    allocation: 16384,
    nodes: 128,
    depth: 8,
};

/// Application-owned protocol for one named Wire call.
pub trait Contract {
    /// Exported canister method.
    const NAME: &'static str;
    /// Stable Wire method identity.
    const ID: u64;
    /// Complete request budget.
    const REQUEST: wire::Limits;
    /// Complete reply budget.
    const REPLY: wire::Limits;
    /// Public input shape, including unit for empty requests.
    type Input: serde::Serialize + serde::de::DeserializeOwned;
    /// Public reply shape.
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    /// Encode the request using this method's directional frame and limits.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn encode_request(value: &Self::Input) -> Result<Vec<u8>, wire::Error> {
        wire::encode(
            value,
            wire::Encoding::Frame {
                method: Self::ID,
                direction: wire::Direction::Request,
            },
            Self::REQUEST,
        )
    }

    /// Decode the request using this method's directional frame and limits.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn decode_request(bytes: &[u8]) -> Result<Self::Input, wire::Error> {
        wire::decode(
            bytes,
            wire::Encoding::Frame {
                method: Self::ID,
                direction: wire::Direction::Request,
            },
            Self::REQUEST,
        )
    }

    /// Encode the reply using this method's directional frame and limits.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn encode_reply(value: &Self::Output) -> Result<Vec<u8>, wire::Error> {
        wire::encode(
            value,
            wire::Encoding::Frame {
                method: Self::ID,
                direction: wire::Direction::Response,
            },
            Self::REPLY,
        )
    }

    /// Decode the reply using this method's directional frame and limits.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn decode_reply(bytes: &[u8]) -> Result<Self::Output, wire::Error> {
        wire::decode(
            bytes,
            wire::Encoding::Frame {
                method: Self::ID,
                direction: wire::Direction::Response,
            },
            Self::REPLY,
        )
    }
}

#[doc(hidden)]
#[must_use]
pub const fn same_name(left: &str, right: &str) -> bool {
    let (a, b) = (left.as_bytes(), right.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
