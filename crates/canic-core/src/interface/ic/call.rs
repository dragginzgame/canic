use crate::cdk::{call::Call as IcCall, candid::Principal};

///
/// Call
/// Thin wrapper around `ic_cdk::api::call::Call`.
///

pub struct Call;

impl Call {
    /// Create a call builder that will be awaited without cycle limits.
    #[must_use]
    pub fn unbounded_wait(canister_id: Principal, method: &str) -> IcCall<'_, '_> {
        IcCall::unbounded_wait(canister_id, method)
    }
}
