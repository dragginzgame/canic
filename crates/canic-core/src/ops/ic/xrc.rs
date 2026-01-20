use crate::{
    InternalError,
    cdk::{
        env::nns::EXCHANGE_RATE_CANISTER,
        spec::standards::xrc::{ExchangeRate, GetExchangeRateRequest, GetExchangeRateResult},
    },
    ops::ic::{IcOpsError, call::CallOps},
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
        let response = CallOps::unbounded_wait(*EXCHANGE_RATE_CANISTER, "get_exchange_rate")
            .with_cycles(cycles)
            .try_with_arg(req)?
            .execute()
            .await?;

        let res: GetExchangeRateResult = response.candid()?;

        match res {
            GetExchangeRateResult::Ok(rate) => Ok(rate),
            GetExchangeRateResult::Err(err) => Err(XrcOpsError::Rejected {
                reason: format!("{err:?}"),
            }
            .into()),
        }
    }
}
