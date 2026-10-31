//! Module: ops::ic::call
//!
//! Responsibility: attribute framework call attempts and project transport failures.
//! Does not own: request builders, codecs, physical execution, retries or settlement.
//! Boundary: consumes an upstream prepared request once through Platform's runtime.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::metrics::{
        PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
        PlatformCallMetricSurface,
    },
    infra::ic::IcInfraError,
    ops::{
        OpsError,
        runtime::metrics::{
            inter_canister_call::InterCanisterCallMetrics, platform_call::PlatformCallMetrics,
        },
    },
};
use ic_call::WaitPolicy;

/// Execute one framework request, counting an attempt only when this future is polled.
///
/// Attempts include Platform submission refusals; they do not prove a remote effect.
/// The caller supplies exact protocol bytes, wait policy and cycle attachment. No
/// additional await follows the matching callback, preserving refund observation.
pub async fn execute(request: ic_call::Call<'_, '_>) -> Result<ic_call::Response, InternalError> {
    let mode = match request.wait_policy() {
        WaitPolicy::Bounded { .. } => PlatformCallMetricMode::BoundedWait,
        WaitPolicy::Unbounded => PlatformCallMetricMode::UnboundedWait,
    };
    InterCanisterCallMetrics::record_call(Principal::from(request.target()), request.method());
    record(mode, PlatformCallMetricOutcome::Started);
    let result = request.execute().await;
    record(
        mode,
        if result.is_ok() {
            PlatformCallMetricOutcome::Completed
        } else {
            PlatformCallMetricOutcome::Failed
        },
    );
    result
        .map_err(IcInfraError::from)
        .map_err(|error| OpsError::from(error).into())
}

fn record(mode: PlatformCallMetricMode, outcome: PlatformCallMetricOutcome) {
    let reason = match outcome {
        PlatformCallMetricOutcome::Failed => PlatformCallMetricReason::Infra,
        _ => PlatformCallMetricReason::Ok,
    };
    PlatformCallMetrics::record(PlatformCallMetricSurface::Generic, mode, outcome, reason);
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::runtime::metrics::system::SystemMetrics;
    use ic_call::Call;

    #[test]
    fn unpolled_execution_has_no_call_metrics_or_platform_effect() {
        InterCanisterCallMetrics::reset();
        PlatformCallMetrics::reset();
        SystemMetrics::reset();
        let bytes = [1, 2, 3];
        let request = Call::new(
            Principal::anonymous().into(),
            String::from("owned_method"),
            WaitPolicy::Bounded {
                timeout_seconds: 17,
            },
        )
        .with_raw_args(&bytes)
        .with_cycles(42);
        drop(execute(request));
        assert!(InterCanisterCallMetrics::snapshot().entries.is_empty());
        assert!(PlatformCallMetrics::snapshot().is_empty());
        assert_eq!(SystemMetrics::count().exact_total(), Ok(0));
    }
}
