use crate::spec::prelude::*;

///
/// AssetClass
/// XRC asset class discriminator.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AssetClass {
    Cryptocurrency,
    FiatCurrency,
}

///
/// Asset
/// XRC asset descriptor used in exchange rate queries.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct Asset {
    pub symbol: String,
    pub class: AssetClass,
}

///
/// GetExchangeRateRequest
/// Request payload for XRC `get_exchange_rate`.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct GetExchangeRateRequest {
    pub base_asset: Asset,
    pub quote_asset: Asset,
    pub timestamp: Option<u64>,
}

///
/// ExchangeRateMetadata
/// Metadata attached to a returned exchange rate.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct ExchangeRateMetadata {
    pub decimals: u32,
    pub base_asset_num_queried_sources: u32,
    pub base_asset_num_received_rates: u32,
    pub quote_asset_num_queried_sources: u32,
    pub quote_asset_num_received_rates: u32,
    pub standard_deviation: u64,
    pub forex_timestamp: Option<u64>,
}

///
/// ExchangeRate
/// Returned exchange rate record.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct ExchangeRate {
    pub rate: u64,
    pub metadata: ExchangeRateMetadata,
}

///
/// ExchangeRateError
/// Error variants returned by XRC.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum ExchangeRateError {
    AnonymousPrincipalNotAllowed,
    CryptoQuoteAssetNotFound,
    FailedToAcceptCycles,
    ForexBaseAssetNotFound,
    CryptoBaseAssetNotFound,
    StablecoinRateTooFewRates,
    ForexAssetsNotFound,
    InconsistentRatesReceived,
    RateLimited,
    StablecoinRateZeroRate,
    Other(String),
    NotEnoughCycles,
    ForexInvalidTimestamp,
    NotEnoughRates,
    ForexQuoteAssetNotFound,
    StablecoinRateNotFound,
}

///
/// GetExchangeRateResult
/// Result envelope for XRC `get_exchange_rate`.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum GetExchangeRateResult {
    Ok(ExchangeRate),
    Err(ExchangeRateError),
}
