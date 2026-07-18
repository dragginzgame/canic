use clap::ValueEnum;
use serde::Serialize;

///
/// MetricsKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(super) enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

///
/// MetricsReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MetricsReport {
    pub(super) deployment: String,
    pub(super) environment: String,
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
    pub(super) status: MetricsCanisterStatus,
    pub(super) entries: Vec<MetricEntry>,
    pub(super) error: Option<String>,
}

///
/// MetricsCanisterStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MetricsCanisterStatus {
    Empty,
    Error,
    Ok,
    Unavailable,
}

impl MetricsCanisterStatus {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Error => "error",
            Self::Ok => "ok",
            Self::Unavailable => "unavailable",
        }
    }
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
