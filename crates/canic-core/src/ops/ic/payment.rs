//! Payment and pricing helpers (IC edge).
//!
//! This module hosts unit conversions and pricing helpers using `rust_decimal`
//! via `canic-types`.

use crate::{Error, cdk::utils::time::now_secs, dto::payment::PriceQuote, types::Decimal};
use num_traits::ToPrimitive;
use rust_decimal::Decimal as RustDecimal;

/// ICP base unit scale.
pub const E8S_PER_ICP: u64 = 100_000_000;

///
/// usd_to_token_amount
/// Converts a USD amount into token base units at 1:1 peg.
///

pub fn usd_to_token_amount(usd_amount: Decimal, token_decimals: u8) -> Result<u64, Error> {
    let base_units = 10_u64
        .checked_pow(u32::from(token_decimals))
        .ok_or_else(|| Error::custom("token decimals overflow"))?;

    let amount = usd_amount.0 * RustDecimal::from(base_units);
    amount
        .to_u64()
        .ok_or_else(|| Error::custom("token amount overflow"))
}

///
/// usd_to_icp_e8s
/// Converts USD to ICP e8s using a USD/ICP rate.
///

pub fn usd_to_icp_e8s(usd_amount: Decimal, usd_per_icp: Decimal) -> Result<u64, Error> {
    if usd_per_icp.0.is_zero() {
        return Err(Error::custom("usd_per_icp must be non-zero"));
    }

    let icp = usd_amount.0 / usd_per_icp.0;
    let e8s = icp * RustDecimal::from(E8S_PER_ICP);
    e8s.to_u64()
        .ok_or_else(|| Error::custom("icp_e8s overflow"))
}

///
/// usd_per_icp
/// Returns a USD/ICP rate and a timestamp.
///
/// Today this returns a deterministic fallback while oracle integration is finalized.
///

#[allow(clippy::unused_async)]
pub async fn usd_per_icp() -> Result<(Decimal, u64), Error> {
    Ok((Decimal(RustDecimal::from(5u64)), now_secs()))
}

///
/// price_quote
/// Returns a Canic-specific pricing envelope suitable for frontends.
///

pub async fn price_quote(usd_amount: Decimal) -> Result<PriceQuote, Error> {
    let (usd_per_icp, timestamp_seconds) = usd_per_icp().await?;
    let icp_e8s = usd_to_icp_e8s(usd_amount, usd_per_icp)?;

    Ok(PriceQuote {
        usd_amount,
        icp_e8s,
        usd_per_icp,
        timestamp_seconds,
    })
}
