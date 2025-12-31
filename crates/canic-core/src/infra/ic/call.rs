use crate::{cdk::call::Call as IcCall, infra::prelude::*};

///
/// Call
/// Wrapper around `ic_cdk::api::call::Call`.
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> IcCall<'_, '_> {
        let canister_id: Principal = canister_id.into();

        IcCall::bounded_wait(canister_id, method)
    }

    /// Create a call builder that will be awaited without cycle limits.
    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> IcCall<'_, '_> {
        let canister_id: Principal = canister_id.into();

        IcCall::unbounded_wait(canister_id, method)
    }
}
