//! Named application Wire endpoints with Platform execution.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use ic_platform_runtime as runtime;
#[doc(hidden)]
pub use ic_wire as wire;

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

/// Observed failure of one outbound application call.
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub enum CallError {
    /// Request refused before submission.
    Request(wire::Error),
    /// Transport could not submit the call.
    Dispatch(runtime::call::SubmitError),
    /// Remote rejection; does not authorize retry.
    Rejected(runtime::call::Rejection),
    /// The transport could not establish the remote outcome.
    Unknown(runtime::call::Rejection),
    /// Returned bytes did not satisfy the declared response contract.
    Reply(wire::Error),
}

/// Submit once to the explicitly selected target using a bounded wait.
/// Platform captures the raw reply before Wire enforces the contract limits.
/// # Errors
/// Returns the observed stage. The application owns reconciliation.
#[cfg(target_arch = "wasm32")]
pub async fn call<M: Contract>(
    target: ic_platform_types::Principal,
    seconds: u32,
    input: &M::Input,
) -> Result<M::Output, CallError> {
    use runtime::call::{Call, Outcome, Wait};
    let request = M::encode_request(input).map_err(CallError::Request)?;
    let result = Call::new(target, M::NAME, Wait::Bounded { seconds })
        .owned_args(request)
        .execute()
        .await
        .map_err(CallError::Dispatch)?;
    match result.into_parts().0 {
        Outcome::Reply(bytes) => M::decode_reply(&bytes).map_err(CallError::Reply),
        Outcome::Rejected(reason) => Err(CallError::Rejected(reason)),
        Outcome::OutcomeUnknown(reason) => Err(CallError::Unknown(reason)),
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

/// Admit the request before Platform copies the argument. Session views are
/// receiver-produced authority and cannot be decoded as endpoint inputs.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub fn admit_request<T: 'static>(maximum: usize) -> Result<(), String> {
    #[cfg(feature = "auth")]
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<crate::auth::protocol::SessionView>() {
        return Err("a session parameter requires an explicit scope".into());
    }
    if runtime::system::argument_size() > maximum {
        return Err("request byte limit exceeded".into());
    }
    Ok(())
}

/// Trap a refused execution stage in its current message.
/// # Panics
/// Always traps.
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
