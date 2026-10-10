//! Bounded Canic transports for native delegation validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    auth::{
        ApplicationSessionAuditResponse, ApplicationSessionCommand,
        ApplicationSessionCommandResponse, ApplicationSessionStatus, DelegatedToken,
        DelegatedTokenGetRequest, DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse,
    },
    metrics::MetricEntry,
    page::{Page, PageRequest},
    role::MetricsStatusRequest,
};

/// Bounded CanisterCommand selectors for native delegation validation.
#[derive(candid::CandidType)]
#[expect(
    clippy::large_enum_variant,
    reason = "the fixture mirrors the exact generated managed command Candid"
)]
pub enum CanisterCommand {
    ApplicationSession(ApplicationSessionCommand),
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}

/// Bounded CanisterCommandResponse selectors for native delegation validation.
#[derive(candid::CandidType, serde::Deserialize)]
pub enum CanisterCommandResponse {
    ApplicationSession(ApplicationSessionCommandResponse),
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}

/// Bounded CanisterStatusRequest selectors for native delegation validation.
#[derive(candid::CandidType)]
pub enum CanisterStatusRequest {
    ApplicationSession,
    ApplicationSessionAudit(PageRequest),
    DelegatedToken(DelegatedTokenGetRequest),
    Metrics(MetricsStatusRequest),
}

/// Bounded CanisterStatusResponse selectors for native delegation validation.
#[derive(candid::CandidType, serde::Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the fixture mirrors the exact generated managed status Candid"
)]
pub enum CanisterStatusResponse {
    ApplicationSession(ApplicationSessionStatus),
    ApplicationSessionAudit(ApplicationSessionAuditResponse),
    DelegatedToken(DelegatedToken),
    Metrics(Page<MetricEntry>),
}
