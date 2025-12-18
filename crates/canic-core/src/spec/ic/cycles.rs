use crate::spec::prelude::*;

///
/// IcpXdrConversionRate
/// Canonical payload from the cycles minting canister describing ICP/XDR rate.
///

#[derive(CandidType, Debug, Deserialize)]
pub struct IcpXdrConversionRate {
    pub timestamp_seconds: u64,
    pub xdr_permyriad_per_icp: u64,
}

///
/// IcpXdrConversionRateResponse
/// Wrapper around the rate record returned by `get_icp_xdr_conversion_rate`.
///

#[derive(CandidType, Debug, Deserialize)]
pub struct IcpXdrConversionRateResponse {
    pub data: IcpXdrConversionRate,
}

///
/// IcpXdrConversionRateCertifiedResponse
/// Certified wrapper returned by `get_icp_xdr_conversion_rate` (when available).
///

#[derive(CandidType, Debug, Deserialize)]
pub struct IcpXdrConversionRateCertifiedResponse {
    pub data: IcpXdrConversionRate,
    pub hash_tree: ByteBuf,
    pub certificate: ByteBuf,
}

///
/// NotifyTopUpArgs
/// Arguments expected by the cycles canister when notifying a top-up.
///

#[derive(CandidType, Deserialize)]
pub struct NotifyTopUpArgs {
    pub block_index: u64,
    pub canister_id: Principal,
}
