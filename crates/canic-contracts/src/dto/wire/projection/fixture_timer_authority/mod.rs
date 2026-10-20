//! Bounded Canic transports for timer authority validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    cycles::{CycleTopupEvent, CycleTrackerEntry},
    metrics::MetricEntry,
    page::{Page, PageRequest},
    public_status::{
        PublicHealth, PublicHistoryRequest, PublicHistorySnapshot, PublicMetricsRequest,
        PublicMetricsSnapshot,
    },
    role::{CycleBalanceStatusResponse, MetricsStatusRequest},
    runtime::CanicRuntimeStatus,
};

/// Bounded RoleStatusRequest selectors for timer authority validation.
#[derive(ic_platform::candid::contract::CandidType, Clone)]
#[candid_path("ic_platform::candid::contract")]
pub enum RoleStatusRequest {
    CycleBalance,
    CycleHistory(PageRequest),
    CycleTopups(PageRequest),
    Metrics(MetricsStatusRequest),
    Runtime,
}

/// Bounded RoleStatusResponse selectors for timer authority validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RoleStatusResponse {
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    CycleTopups(Page<CycleTopupEvent>),
    Metrics(Page<MetricEntry>),
    Runtime(Box<CanicRuntimeStatus>),
}

/// Bounded PublicStatusRequest selectors for timer authority validation.
#[derive(ic_platform::candid::contract::CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum PublicStatusRequest {
    Health,
    Metrics(PublicMetricsRequest),
    History(PublicHistoryRequest),
}

/// Bounded PublicStatusResponse selectors for timer authority validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum PublicStatusResponse {
    Health(PublicHealth),
    Metrics(PublicMetricsSnapshot),
    History(PublicHistorySnapshot),
}
