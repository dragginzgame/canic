//! Named application Wire endpoints with Platform execution.
mod actor;
#[cfg(test)]
mod tests;

pub use actor::{CallError, admit_request, call, call_unbounded, call_with_cycles};

#[doc(hidden)]
pub use ic_wire as wire;

/// Abort the current endpoint execution on a refused stage.
/// # Panics
/// Traps the current IC message or panics when executing a native handler.
#[doc(hidden)]
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

/// Default Wire budgets for application requests and replies.
pub const LIMITS: wire::Limits = wire::Limits {
    bytes: 16 * 1024,
    allocation: 384 * 1024,
    nodes: 4_096,
    depth: 16,
};

const fn request_frame<M: Contract + ?Sized>() -> wire::Encoding {
    wire::Encoding::Frame {
        method: M::ID,
        direction: wire::Direction::Request,
    }
}

/// Application-owned protocol for one named Wire call.
pub trait Contract {
    /// Exported canister method.
    const NAME: &'static str;
    /// Stable Wire method identity.
    const ID: u64;
    /// Request budget, defaulting to the standard endpoint limits.
    const REQUEST: wire::Limits = LIMITS;
    /// Reply budget, defaulting to the request budget.
    const REPLY: wire::Limits = Self::REQUEST;
    /// Public input shape, including unit for empty requests.
    type Input: serde::Serialize + serde::de::DeserializeOwned;
    /// Public reply shape.
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    /// Measure the complete request frame without allocating output or issuing a call.
    /// A borrowed representation must serialize identically to `Self::Input`.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn request_len<T: serde::Serialize + ?Sized>(value: &T) -> Result<usize, wire::Error> {
        wire::encoded_len(value, request_frame::<Self>(), Self::REQUEST)
    }

    /// Encode the request using this method's directional frame and limits.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn encode_request(value: &Self::Input) -> Result<Vec<u8>, wire::Error> {
        wire::encode(value, request_frame::<Self>(), Self::REQUEST)
    }

    /// Decode the request using this method's directional frame and limits.
    /// # Errors
    /// Propagates the Wire codec refusal.
    fn decode_request(bytes: &[u8]) -> Result<Self::Input, wire::Error> {
        wire::decode(bytes, request_frame::<Self>(), Self::REQUEST)
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

// Inference has one candidate for a plain contract. A role method has a second
// candidate, so a plain mount fails to type-check even through a type alias.
#[doc(hidden)]
pub trait PlainMount<Marker> {
    const CHECK: () = ();
}
impl<M: Contract> PlainMount<()> for M {}
#[cfg(all(feature = "topology", feature = "method"))]
impl<M: crate::topology::attestation::RoleMethod> PlainMount<AttestedMount> for M {}
#[cfg(all(feature = "topology", feature = "method"))]
#[doc(hidden)]
pub enum AttestedMount {}

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
