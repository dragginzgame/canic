use crate::{dto::prelude::*, types::Decimal};

///
/// PriceQuote
/// Canic-specific pricing envelope for frontends and integrators.
///
/// - `usd_amount`: USD amount (candid `text`, backed by `rust_decimal`).
/// - `icp_e8s`: ICP amount in e8s.
/// - `usd_per_icp`: USD per ICP exchange rate (candid `text`, backed by `rust_decimal`).
/// - `timestamp_seconds`: UNIX epoch time in seconds.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PriceQuote {
    pub usd_amount: Decimal,
    pub icp_e8s: u64,
    pub usd_per_icp: Decimal,
    pub timestamp_seconds: u64,
}
