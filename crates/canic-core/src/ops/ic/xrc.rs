use crate::{
    Error, ThisError,
    cdk::{
        env::nns::EXCHANGE_RATE_CANISTER,
        spec::ic::xrc::{ExchangeRate, GetExchangeRateRequest, GetExchangeRateResult},
    },
    ops::ic::{IcOpsError, call::Call},
};

/// Default cycles to attach to XRC calls.
pub const DEFAULT_XRC_CYCLES: u128 = 0;

///
/// XrcOpsError
///

#[derive(Debug, ThisError)]
pub enum XrcOpsError {
    #[error("xrc rejected exchange rate request: {reason}")]
    Rejected { reason: String },
}

impl From<XrcOpsError> for Error {
    fn from(err: XrcOpsError) -> Self {
        IcOpsError::from(err).into()
    }
}

pub async fn get_exchange_rate(
    req: GetExchangeRateRequest,
    cycles: u128,
) -> Result<ExchangeRate, Error> {
    let response = Call::unbounded_wait(*EXCHANGE_RATE_CANISTER, "get_exchange_rate")
        .with_cycles(cycles)
        .with_arg(req)
        .await
        .map_err(IcOpsError::from)?;

    let res: GetExchangeRateResult = response.candid().map_err(IcOpsError::from)?;

    match res {
        GetExchangeRateResult::Ok(rate) => Ok(rate),
        GetExchangeRateResult::Err(err) => Err(XrcOpsError::Rejected {
            reason: format!("{err:?}"),
        }
        .into()),
    }
}
