//! Exchange Rate Canister (XRC) helpers.

use crate::{
    Error,
    cdk::spec::ic::xrc::{ExchangeRate, GetExchangeRateRequest},
    ops::ic::xrc as xrc_ops,
};

/// get_exchange_rate
/// Calls XRC `get_exchange_rate` using the default cycle budget.
#[expect(dead_code)]
pub(crate) async fn get_exchange_rate(req: GetExchangeRateRequest) -> Result<ExchangeRate, Error> {
    xrc_ops::get_exchange_rate(req, xrc_ops::DEFAULT_XRC_CYCLES).await
}
