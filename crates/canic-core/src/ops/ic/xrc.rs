use crate::{
    InternalError,
    cdk::{
        env::nns::EXCHANGE_RATE_CANISTER,
        spec::standards::xrc::{ExchangeRate, GetExchangeRateRequest, GetExchangeRateResult},
    },
    ops::{
        ic::{IcOpsError, call::CallOps},
        runtime::metrics::platform_call::{
            PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
            PlatformCallMetricSurface, PlatformCallMetrics,
        },
    },
};
use thiserror::Error as ThisError;

///
/// XrcOpsError
///

#[derive(Debug, ThisError)]
pub enum XrcOpsError {
    #[error("xrc rejected exchange rate request: {reason}")]
    Rejected { reason: String },
}

impl From<XrcOpsError> for InternalError {
    fn from(err: XrcOpsError) -> Self {
        IcOpsError::from(err).into()
    }
}

///
/// XrcOps
///

pub struct XrcOps;

impl XrcOps {
    /// Default cycles to attach to XRC calls.
    pub const DEFAULT_XRC_CYCLES: u128 = 0;

    pub async fn get_exchange_rate(
        req: GetExchangeRateRequest,
        cycles: u128,
    ) -> Result<ExchangeRate, InternalError> {
        record_xrc_call(
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let builder = match CallOps::unbounded_wait(*EXCHANGE_RATE_CANISTER, "get_exchange_rate")
            .with_cycles(cycles)
            .with_arg(req)
        {
            Ok(builder) => builder,
            Err(err) => {
                record_xrc_call(
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::CandidEncode,
                );
                return Err(err);
            }
        };
        let response = match builder.execute().await {
            Ok(response) => response,
            Err(err) => {
                record_xrc_call(
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::Infra,
                );
                return Err(err);
            }
        };

        let res: GetExchangeRateResult = match response.candid() {
            Ok(res) => res,
            Err(err) => {
                record_xrc_call(
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::CandidDecode,
                );
                return Err(err);
            }
        };

        match res {
            GetExchangeRateResult::Ok(rate) => {
                record_xrc_call(
                    PlatformCallMetricOutcome::Completed,
                    PlatformCallMetricReason::Ok,
                );
                Ok(rate)
            }
            GetExchangeRateResult::Err(err) => {
                record_xrc_call(
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::Rejected,
                );
                Err(XrcOpsError::Rejected {
                    reason: format!("{err:?}"),
                }
                .into())
            }
        }
    }
}

// Record one XRC metric with no asset, quote, or provider dimensions.
fn record_xrc_call(outcome: PlatformCallMetricOutcome, reason: PlatformCallMetricReason) {
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Xrc,
        PlatformCallMetricMode::UnboundedWait,
        outcome,
        reason,
    );
}
