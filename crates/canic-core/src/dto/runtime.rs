use crate::dto::prelude::*;

pub use crate::domain::runtime::{
    FailureSeverity, RuntimeCheckStatus, RuntimeDiagnosticSeverity, RuntimeFieldVisibility,
    RuntimeStateDomainStatus,
};

pub const RUNTIME_INTROSPECTION_SCHEMA_VERSION: u32 = 1;

//
// CanicHealthStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicHealthStatus {
    pub schema_version: u32,
    pub status: HealthStatus,
    pub observed_at_ns: Option<u64>,
    pub checks: Vec<RuntimeCheck>,
}

//
// CanicReadinessStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicReadinessStatus {
    pub schema_version: u32,
    pub role: Option<String>,
    pub status: ReadinessStatus,
    pub observed_at_ns: u64,
    pub checks: Vec<RuntimeCheck>,
    pub blockers: Vec<RuntimeDiagnostic>,
    pub warnings: Vec<RuntimeDiagnostic>,
}

//
// CanicRuntimeStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicRuntimeStatus {
    pub schema_version: u32,
    pub observed_at_ns: u64,
    pub canister_id: Principal,
    pub role: Option<String>,
    pub root: Option<Principal>,
    pub network: Option<String>,
    pub build: RuntimeBuildInfo,
    pub features: Vec<RuntimeFeatureStatus>,
    pub topology: Option<RuntimeTopologyStatus>,
    pub timers: Vec<CanicTimerStatus>,
    pub state: Option<RuntimeStateSummary>,
    pub auth: Option<RuntimeAuthStatusSummary>,
    pub blob_storage: Option<RuntimeBlobStorageStatusSummary>,
    pub recent_failures: Vec<RecentFailure>,
    pub visibility: Vec<RuntimeVisibilityEntry>,
    pub readiness: CanicReadinessStatus,
    pub status: RuntimeStatus,
}

//
// RuntimeBuildInfo
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeBuildInfo {
    pub package_name: String,
    pub package_version: String,
    pub canic_version: String,
    pub canister_version: u64,
}

//
// RuntimeCheck
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeCheck {
    pub category: String,
    pub code: String,
    pub status: RuntimeCheckStatus,
    pub subject: String,
    pub detail: String,
    pub next: Option<String>,
    pub source: String,
}

//
// RuntimeDiagnostic
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeDiagnostic {
    pub category: String,
    pub code: String,
    pub severity: RuntimeDiagnosticSeverity,
    pub subject: String,
    pub detail: String,
    pub next: Option<String>,
    pub source: String,
}

//
// RuntimeFeatureStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeFeatureStatus {
    pub name: String,
    pub enabled: bool,
    pub visibility: RuntimeFieldVisibility,
    pub source: String,
}

//
// RuntimeTopologyStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeTopologyStatus {
    pub root: Option<Principal>,
    pub parent: Option<Principal>,
    pub subnet: Option<Principal>,
    pub source: String,
}

//
// CanicTimerStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicTimerStatus {
    pub name: String,
    pub subsystem: String,
    pub status: TimerStatus,
    pub enabled: bool,
    pub registered: bool,
    pub last_success_at_ns: Option<u64>,
    pub last_failure_at_ns: Option<u64>,
    pub next_due_at_ns: Option<u64>,
    pub consecutive_failures: u64,
    pub last_error_code: Option<String>,
    pub last_error_summary: Option<String>,
}

//
// RuntimeStateSummary
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeStateSummary {
    pub manifest_schema_version: u32,
    pub domains: Vec<RuntimeStateDomainSummary>,
    pub total_stable_memory_pages: Option<u64>,
}

//
// RuntimeStateDomainSummary
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeStateDomainSummary {
    pub domain: String,
    pub version: u32,
    pub storage: String,
    pub memory_id: Option<u8>,
    pub status: RuntimeStateDomainStatus,
}

//
// RuntimeAuthStatusSummary
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeAuthStatusSummary {
    pub auth_features: Vec<RuntimeFeatureStatus>,
}

//
// RuntimeBlobStorageStatusSummary
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeBlobStorageStatusSummary {
    pub blob_storage_features: Vec<RuntimeFeatureStatus>,
}

//
// RecentFailure
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecentFailure {
    pub occurred_at_ns: u64,
    pub subsystem: String,
    pub code: String,
    pub severity: FailureSeverity,
    pub summary: String,
    pub correlation_id: Option<String>,
    pub redacted: bool,
}

//
// RuntimeVisibilityEntry
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeVisibilityEntry {
    pub field: String,
    pub visibility: RuntimeFieldVisibility,
}

//
// Status enums
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    #[serde(alias = "Healthy")]
    Healthy,
    #[serde(alias = "Degraded")]
    Degraded,
    #[serde(alias = "Unhealthy")]
    Unhealthy,
    #[serde(alias = "Unknown")]
    Unknown,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    #[serde(alias = "Ready")]
    Ready,
    #[serde(alias = "Degraded")]
    Degraded,
    #[serde(alias = "NotReady")]
    NotReady,
    #[serde(alias = "NotEvaluated")]
    NotEvaluated,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    #[serde(alias = "Ok")]
    Ok,
    #[serde(alias = "Degraded")]
    Degraded,
    #[serde(alias = "Failing")]
    Failing,
    #[serde(alias = "Unknown")]
    Unknown,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    #[serde(alias = "Healthy")]
    Healthy,
    #[serde(alias = "Delayed")]
    Delayed,
    #[serde(alias = "Failing")]
    Failing,
    #[serde(alias = "Disabled")]
    Disabled,
    #[serde(alias = "NotRegistered")]
    NotRegistered,
    #[serde(alias = "Unknown")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn runtime_enums_roundtrip_candid_with_runtime_variant_labels() {
        assert_enum_candid_contract(HealthStatus::Unknown);
        assert_enum_candid_contract(ReadinessStatus::NotEvaluated);
        assert_enum_candid_contract(RuntimeStatus::Failing);
        assert_enum_candid_contract(TimerStatus::NotRegistered);
        assert_enum_candid_contract(FailureSeverity::Critical);
        assert_enum_candid_contract(RuntimeCheckStatus::NotEvaluated);
        assert_enum_candid_contract(RuntimeDiagnosticSeverity::Unsupported);
        assert_enum_candid_contract(RuntimeFieldVisibility::OperatorOnly);
        assert_enum_candid_contract(RuntimeStateDomainStatus::NotEvaluated);
    }

    fn assert_enum_candid_contract<T>(value: T)
    where
        T: CandidType + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = Encode!(&value).expect("encode runtime enum");
        let decoded = Decode!(&bytes, T).expect("decode runtime enum");

        assert_eq!(decoded, value);
    }
}
