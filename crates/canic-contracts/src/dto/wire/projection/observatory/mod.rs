//! Passive observatory transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    public_status::{
        PublicHistoryRequest, PublicHistorySnapshot, PublicMetricsRequest, PublicMetricsSnapshot,
    },
    root::RootFundingStatusResponse,
    template::WasmStoreStatusResponse,
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded RootFundingResponse wire projection for observatory.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootFundingResponse {
    Funding(RootFundingStatusResponse),
}

/// Bounded StoreRequest wire projection for observatory.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StoreRequest {
    Storage,
}

/// Bounded StoreResponse wire projection for observatory.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StoreResponse {
    Storage(WasmStoreStatusResponse),
}

/// Bounded MetricRequest wire projection for observatory.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum MetricRequest {
    Metrics(PublicMetricsRequest),
    History(PublicHistoryRequest),
}

/// Bounded MetricResponse wire projection for observatory.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum MetricResponse {
    Metrics(PublicMetricsSnapshot),
    History(PublicHistorySnapshot),
}
