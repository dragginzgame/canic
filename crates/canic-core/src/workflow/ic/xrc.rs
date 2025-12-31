//! Exchange Rate Canister (XRC) helpers.

use crate::{
    Error, ThisError,
    cdk::{
        env::nns::EXCHANGE_RATE_CANISTER,
        spec::ic::xrc::{
            ExchangeRate, ExchangeRateError, GetExchangeRateRequest, GetExchangeRateResult,
        },
    },
    infra::InfraError,
    ops::ic::call::Call,
    workflow::ic::IcWorkflowError,
};

/// Default cycles to attach to XRC calls.
///
/// XRC charges cycles for exchange rate queries; callers may want to tune this
/// based on the environment and query type.
pub const DEFAULT_XRC_CYCLES: u128 = 0;

///
/// XrcWorkflowError
///

#[derive(Debug, ThisError)]
pub enum XrcWorkflowError {
    #[error("xrc get_exchange_rate failed: {0:?}")]
    ExchangeRateRejected(ExchangeRateError),
}

impl From<XrcWorkflowError> for Error {
    fn from(err: XrcWorkflowError) -> Self {
        IcWorkflowError::from(err).into()
    }
}

///
/// get_exchange_rate
/// Calls XRC `get_exchange_rate` and normalizes errors into `crate::Error`.
///

#[expect(dead_code)]
pub(crate) async fn get_exchange_rate(req: GetExchangeRateRequest) -> Result<ExchangeRate, Error> {
    get_exchange_rate_with_cycles(req, DEFAULT_XRC_CYCLES).await
}

///
/// get_exchange_rate_with_cycles
/// Calls XRC `get_exchange_rate` while attaching cycles.
///

pub(crate) async fn get_exchange_rate_with_cycles(
    req: GetExchangeRateRequest,
    cycles: u128,
) -> Result<ExchangeRate, Error> {
    let response = Call::unbounded_wait(*EXCHANGE_RATE_CANISTER, "get_exchange_rate")
        .with_cycles(cycles)
        .with_arg(req)
        .await
        .map_err(InfraError::from)?;

    let res: GetExchangeRateResult = response.candid().map_err(InfraError::from)?;

    match res {
        GetExchangeRateResult::Ok(rate) => Ok(rate),

        GetExchangeRateResult::Err(err) => Err(XrcWorkflowError::ExchangeRateRejected(err).into()),
    }
}
