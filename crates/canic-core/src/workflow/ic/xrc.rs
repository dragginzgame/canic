//! Module: workflow::ic::xrc
//!
//! Responsibility: expose Exchange Rate Canister workflow helpers.
//! Does not own: XRC call execution, endpoint authorization, or DTO schemas.
//! Boundary: delegates XRC calls to IC ops with workflow-level defaults.

use crate::{
    InternalError,
    cdk::spec::standards::xrc::{ExchangeRate, GetExchangeRateRequest},
    ops::ic::xrc::XrcOps,
};

///
/// XrcWorkflow
///
/// Workflow facade for Exchange Rate Canister operations.
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
