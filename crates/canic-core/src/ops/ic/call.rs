use crate::{
    cdk::{call::Call as IcCall, candid::Principal},
    ops::runtime::metrics::icc::record_icc_call,
};

///
/// Call
/// Wrapper around `ic_cdk::api::call::Call` that records metrics.
///

pub struct Call;

impl Call {
    #[must_use]
    #[expect(dead_code)]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> IcCall<'_, '_> {
        let canister_id: Principal = canister_id.into();

        record_icc_call(canister_id, method);

        IcCall::bounded_wait(canister_id, method)
    }

    /// Create a call builder that will be awaited without cycle limits.
    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> IcCall<'_, '_> {
        let canister_id: Principal = canister_id.into();

        record_icc_call(canister_id, method);

        IcCall::unbounded_wait(canister_id, method)
    }
}
