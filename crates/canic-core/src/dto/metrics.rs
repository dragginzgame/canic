use crate::{dto::prelude::*, ids::AccessMetricKind};

///
/// AccessMetricEntry
/// Snapshot entry pairing an endpoint/stage with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AccessMetricEntry {
    pub endpoint: String,
    pub kind: AccessMetricKind,
    pub count: u64,
}

///
/// EndpointAttemptMetricEntry
/// Public metric entry for endpoint attempt/completion.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointAttemptMetricEntry {
    pub endpoint: String,
    pub attempted: u64,
    pub completed: u64,
}

///
/// EndpointResultMetricEntry
/// Public metric entry for endpoint ok/err outcomes.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointResultMetricEntry {
    pub endpoint: String,
    pub ok: u64,
    pub err: u64,
}

///
/// EndpointHealthView
/// Derived endpoint-level health view joined at read time.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointHealthView {
    pub endpoint: String,
    pub attempted: u64,
    pub denied: u64,
    pub completed: u64,
    pub ok: u64,
    pub err: u64,
}

///
/// HttpMetricEntry
/// Snapshot entry pairing a method/label with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpMetricEntry {
    pub method: String,
    pub label: String,
    pub count: u64,
}

///
/// IccMetricEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct IccMetricEntry {
    pub target: Principal,
    pub method: String,
    pub count: u64,
}

///
/// SystemMetricEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemMetricEntry {
    pub kind: String,
    pub count: u64,
}

///
/// TimerMetricEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct TimerMetricEntry {
    pub mode: String,
    pub delay_ms: u64,
    pub label: String,
    pub count: u64,
}
