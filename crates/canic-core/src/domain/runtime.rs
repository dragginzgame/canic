//! Module: domain::runtime
//!
//! Responsibility: define pure runtime value enums shared by runtime ops,
//! status builders, and endpoint DTOs.
//! Does not own: runtime status DTO structs, endpoint guards, or runtime
//! mutation.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::{Deserialize, Serialize};

// CandidType supports explicit serde(rename) labels, but not rename_all. Keep
// runtime introspection enum labels canonical and alias-free.

///
/// FailureSeverity
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FailureSeverity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "critical")]
    Critical,
}

///
/// RuntimeFieldVisibility
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeFieldVisibility {
    #[serde(rename = "public_safe")]
    PublicSafe,
    #[serde(rename = "operator_only")]
    OperatorOnly,
    #[serde(rename = "controller_only")]
    ControllerOnly,
    #[serde(rename = "feature_gated")]
    FeatureGated,
    #[serde(rename = "disabled")]
    Disabled,
}

///
/// RuntimeCheckStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeCheckStatus {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "not_evaluated")]
    NotEvaluated,
}

///
/// RuntimeDiagnosticSeverity
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeDiagnosticSeverity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "unsupported")]
    Unsupported,
}

///
/// RuntimeStateDomainStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeStateDomainStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "failing")]
    Failing,
    #[serde(rename = "not_evaluated")]
    NotEvaluated,
}

///
/// HealthStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum HealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unhealthy")]
    Unhealthy,
    #[serde(rename = "unknown")]
    Unknown,
}

///
/// ReadinessStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReadinessStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "not_ready")]
    NotReady,
    #[serde(rename = "not_evaluated")]
    NotEvaluated,
}

///
/// RuntimeStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failing")]
    Failing,
    #[serde(rename = "unknown")]
    Unknown,
}

///
/// TimerStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "delayed")]
    Delayed,
    #[serde(rename = "failing")]
    Failing,
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "not_registered")]
    NotRegistered,
    #[serde(rename = "unknown")]
    Unknown,
}

///
/// TimerMode
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum TimerMode {
    Interval,
    Once,
}
