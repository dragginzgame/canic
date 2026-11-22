//! Outgoing calls and ingress admission on the installed actor.
use super::{Contract, wire};
use ic_platform_runtime::{
    self as runtime,
    call::{Call, Completion, Outcome, SubmitError, Wait},
};
use ic_platform_types::Principal;

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
    target: Principal,
    seconds: u32,
    input: &M::Input,
) -> Result<M::Output, CallError> {
    submit::<M>(target, Wait::Bounded { seconds }, input, 0).await
}

/// Submit once and wait without a caller-selected timeout.
/// # Errors
/// Returns the same stage-specific errors as [`call`]. The application owns reconciliation.
pub async fn call_unbounded<M: Contract>(
    target: Principal,
    input: &M::Input,
) -> Result<M::Output, CallError> {
    submit::<M>(target, Wait::Unbounded, input, 0).await
}

/// Submit one paid call using guaranteed response delivery for unaccepted cycles.
/// # Errors
/// Returns the original submission, transport or Wire failure.
pub async fn call_with_cycles<M: Contract>(
    target: Principal,
    input: &M::Input,
    cycles: u128,
) -> Result<M::Output, CallError> {
    submit::<M>(target, Wait::Unbounded, input, cycles).await
}

async fn submit<M: Contract>(
    target: Principal,
    wait: Wait,
    input: &M::Input,
    cycles: u128,
) -> Result<M::Output, CallError> {
    let request = M::encode_request(input).map_err(CallError::Request)?;
    let result = Call::new(target, M::NAME, wait)
        .owned_args(request)
        .cycles(cycles)
        .execute()
        .await;
    decode_result::<M>(result)
}

pub(super) fn decode_result<M: Contract>(
    result: Result<Completion, SubmitError>,
) -> Result<M::Output, CallError> {
    match result.map_err(CallError::Dispatch)?.into_parts().0 {
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
