//! Passive observatory transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::public_status::PublicHistoryRequest;
use crate::dto::public_status::PublicHistorySnapshot;
use crate::dto::public_status::PublicMetricsRequest;
use crate::dto::public_status::PublicMetricsSnapshot;
use crate::dto::root::RootFundingStatusResponse;
use crate::dto::template::WasmStoreStatusResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded RootFundingResponse wire projection for observatory.
#[derive(CandidType, Deserialize)]
pub enum RootFundingResponse {
    Funding(RootFundingStatusResponse),
}

/// Bounded StoreRequest wire projection for observatory.
#[derive(CandidType)]
pub enum StoreRequest {
    Storage,
}

/// Bounded StoreResponse wire projection for observatory.
#[derive(CandidType, Deserialize)]
pub enum StoreResponse {
    Storage(WasmStoreStatusResponse),
}

/// Bounded MetricRequest wire projection for observatory.
#[derive(CandidType)]
pub enum MetricRequest {
    Metrics(PublicMetricsRequest),
    History(PublicHistoryRequest),
}

/// Bounded MetricResponse wire projection for observatory.
#[derive(CandidType, Deserialize)]
pub enum MetricResponse {
    Metrics(PublicMetricsSnapshot),
    History(PublicHistorySnapshot),
}
