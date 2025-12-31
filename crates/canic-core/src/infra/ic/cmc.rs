//! Cycles minting canister (CMC) helpers.

use crate::{
    cdk::{
        env::nns::CYCLES_MINTING_CANISTER,
        spec::ic::cycles::{
            IcpXdrConversionRate, IcpXdrConversionRateCertifiedResponse,
            IcpXdrConversionRateResponse,
        },
    },
    infra::prelude::*,
};

///
/// get_icp_xdr_conversion_rate
/// Fetch the current ICP/XDR conversion rate from the CMC.
///

pub async fn get_icp_xdr_conversion_rate() -> Result<IcpXdrConversionRate, InfraError> {
    let response =
        Call::unbounded_wait(*CYCLES_MINTING_CANISTER, "get_icp_xdr_conversion_rate").await?;

    // The CMC has historically returned both a plain response and a certified
    // response envelope; accept either.
    if let Ok(certified) = candid::decode_one::<IcpXdrConversionRateCertifiedResponse>(&response) {
        return Ok(certified.data);
    }

    let plain = candid::decode_one::<IcpXdrConversionRateResponse>(&response)?;

    Ok(plain.data)
}
