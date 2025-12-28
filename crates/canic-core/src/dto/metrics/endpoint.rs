use crate::cdk::candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// EndpointAttemptMetricEntry
/// Public metric entry for endpoint attempt/completion.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EndpointAttemptMetricEntry {
    pub endpoint: String,
    pub attempted: u64,
    pub completed: u64,
}

///
/// EndpointAttemptMetricsSnapshot
///

pub type EndpointAttemptMetricsSnapshot = Vec<EndpointAttemptMetricEntry>;

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
    pub avg_instr: u64,
    pub total_instr: u64,
}
