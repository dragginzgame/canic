use crate::metrics::MetricsCommandError;
use serde::Serialize;

///
/// MetricsKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricsKind {
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

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Placement => "placement",
            Self::Platform => "platform",
            Self::Runtime => "runtime",
            Self::Security => "security",
            Self::Storage => "storage",
        }
    }

    pub(super) const fn candid_variant(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Placement => "Placement",
            Self::Platform => "Platform",
            Self::Runtime => "Runtime",
            Self::Security => "Security",
            Self::Storage => "Storage",
        }
    }
}

///
/// MetricsReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricsReport {
    pub fleet: String,
    pub network: String,
    pub kind: MetricsKind,
    pub canisters: Vec<MetricsCanisterReport>,
}

///
/// MetricsCanisterReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricsCanisterReport {
    pub role: String,
    pub canister_id: String,
    pub status: String,
    pub entries: Vec<MetricEntry>,
    pub error: Option<String>,
}

///
/// MetricEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricEntry {
    pub labels: Vec<String>,
    pub principal: Option<String>,
    pub value: MetricValue,
}

///
/// MetricValue
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MetricValue {
    Count { count: u64 },
    CountAndU64 { count: u64, value_u64: u64 },
    U128 { value: u128 },
}

impl MetricValue {
    pub(super) const fn is_zero(&self) -> bool {
        match self {
            Self::Count { count } => *count == 0,
            Self::CountAndU64 { count, value_u64 } => *count == 0 && *value_u64 == 0,
            Self::U128 { value } => *value == 0,
        }
    }
}
