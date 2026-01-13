//! Exchange Rate Canister (XRC) helpers.

use crate::{
    InternalError,
    ops::ic::xrc::{ExchangeRate, GetExchangeRateRequest, XrcOps},
};

///
/// XrcWorkflow
///

pub struct XrcWorkflow;

impl XrcWorkflow {
    /// get_exchange_rate
    /// Calls XRC `get_exchange_rate` using the default cycle budget.
    #[expect(dead_code)]
    pub async fn get_exchange_rate(
        req: GetExchangeRateRequest,
    ) -> Result<ExchangeRate, InternalError> {
        XrcOps::get_exchange_rate(req, XrcOps::DEFAULT_XRC_CYCLES).await
    }
}
