//! Outgoing calls and ingress admission on the installed actor.
use super::{Contract, wire};
use ic_platform_runtime as runtime;

/// Observed failure of one outbound application call.
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

/// Admit the request before Platform copies the argument. Session views are
/// receiver-produced authority and cannot be decoded as endpoint inputs.
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
