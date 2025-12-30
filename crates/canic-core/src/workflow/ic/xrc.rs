//! Exchange Rate Canister (XRC) helpers.

use crate::{
    Error,
    cdk::{
        env::nns::EXCHANGE_RATE_CANISTER,
        spec::ic::xrc::{ExchangeRate, GetExchangeRateRequest, GetExchangeRateResult},
    },
    infra::ic::call::Call,
};

/// Default cycles to attach to XRC calls.
///
/// XRC charges cycles for exchange rate queries; callers may want to tune this
/// based on the environment and query type.
pub const DEFAULT_XRC_CYCLES: u128 = 0;

///
/// get_exchange_rate
/// Calls XRC `get_exchange_rate` and normalizes errors into `crate::Error`.
///

pub async fn get_exchange_rate(req: GetExchangeRateRequest) -> Result<ExchangeRate, Error> {
    get_exchange_rate_with_cycles(req, DEFAULT_XRC_CYCLES).await
}

///
/// get_exchange_rate_with_cycles
/// Calls XRC `get_exchange_rate` while attaching cycles.
///

pub async fn get_exchange_rate_with_cycles(
    req: GetExchangeRateRequest,
    cycles: u128,
) -> Result<ExchangeRate, Error> {
    let res: GetExchangeRateResult =
        Call::unbounded_wait(*EXCHANGE_RATE_CANISTER, "get_exchange_rate")
            .with_cycles(cycles)
            .with_arg(req)
            .await?
            .candid()?;

    match res {
        GetExchangeRateResult::Ok(rate) => Ok(rate),
        GetExchangeRateResult::Err(err) => {
            Err(Error::custom(format!("xrc get_exchange_rate: {err:?}")))
        }
    }
}
