use crate::dto::prelude::*;

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
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    Ready,
    Degraded,
    NotReady,
    NotEvaluated,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Ok,
    Degraded,
    Failing,
    Unknown,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    Healthy,
    Delayed,
    Failing,
    Disabled,
    NotRegistered,
    Unknown,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCheckStatus {
    Pass,
    Warn,
    Fail,
    NotEvaluated,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDiagnosticSeverity {
    Info,
    Warning,
    Blocked,
    Unsupported,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFieldVisibility {
    PublicSafe,
    OperatorOnly,
    ControllerOnly,
    FeatureGated,
    Disabled,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStateDomainStatus {
    Ok,
    Warning,
    Failing,
    NotEvaluated,
}
