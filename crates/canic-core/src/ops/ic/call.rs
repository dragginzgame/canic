#![allow(clippy::disallowed_methods)]

use crate::{
    cdk::{call::Call as IcCall, candid::Principal},
    model::metrics::{IccMetrics, SystemMetricKind, SystemMetrics},
};

///
/// Call
/// Wrapper around `ic_cdk::api::call::Call` that records metrics.
///

pub struct Call;

impl Call {
    /// Create a call builder that will be awaited without cycle limits.
    #[must_use]
    pub fn unbounded_wait(canister_id: Principal, method: &str) -> IcCall<'_, '_> {
        SystemMetrics::increment(SystemMetricKind::CanisterCall);
        IccMetrics::increment(canister_id, method);

        IcCall::unbounded_wait(canister_id, method)
    }
}
