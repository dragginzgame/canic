use crate::{
    cdk::call::Call as IcCall,
    model::metrics::{IccMetrics, MetricKind, MetricsState},
    types::Principal,
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
        MetricsState::increment(MetricKind::CanisterCall);
        IccMetrics::increment(canister_id, method);

        IcCall::unbounded_wait(canister_id, method)
    }
}
