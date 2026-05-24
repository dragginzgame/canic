use crate::metrics::MetricsCommandError;
use serde::Serialize;

///
/// MetricsKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

impl MetricsKind {
    pub(super) fn parse(value: &str) -> Result<Self, MetricsCommandError> {
        match value {
            "core" => Ok(Self::Core),
            "placement" => Ok(Self::Placement),
            "platform" => Ok(Self::Platform),
            "runtime" => Ok(Self::Runtime),
            "security" => Ok(Self::Security),
            "storage" => Ok(Self::Storage),
            _ => Err(MetricsCommandError::InvalidKind(value.to_string())),
        }
    }
}

///
/// MetricsReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MetricsReport {
    pub(super) fleet: String,
    pub(super) network: String,
    pub(super) kind: MetricsKind,
    pub(super) canisters: Vec<MetricsCanisterReport>,
}

///
/// MetricsCanisterReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MetricsCanisterReport {
    pub(super) role: String,
    pub(super) canister_id: String,
    pub(super) status: String,
    pub(super) entries: Vec<MetricEntry>,
    pub(super) error: Option<String>,
}

///
/// MetricEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MetricEntry {
    pub(super) labels: Vec<String>,
    pub(super) principal: Option<String>,
    pub(super) value: MetricValue,
}

///
/// MetricValue
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(super) enum MetricValue {
    Count { count: u64 },
    CountAndU64 { count: u64, value_u64: u64 },
    U128 { value: u128 },
}
