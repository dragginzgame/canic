use crate::spec::prelude::*;

///
/// IcpXdrConversionRate
///

#[derive(CandidType, Debug, Deserialize)]
pub struct IcpXdrConversionRate {
    pub timestamp_seconds: u64,
    pub xdr_permyriad_per_icp: u64,
}

///
/// IcpXdrConversionRateResponse
///

#[derive(CandidType, Debug, Deserialize)]
pub struct IcpXdrConversionRateResponse {
    pub data: IcpXdrConversionRate,
}

///
/// NotifyTopUpArgs
///

#[derive(CandidType, Deserialize)]
pub struct NotifyTopUpArgs {
    pub block_index: u64,
    pub canister_id: Principal,
}
