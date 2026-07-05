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

///
/// FailureSeverity
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureSeverity {
    #[serde(alias = "Info")]
    Info,
    #[serde(alias = "Warning")]
    Warning,
    #[serde(alias = "Error")]
    Error,
    #[serde(alias = "Critical")]
    Critical,
}

///
/// RuntimeFieldVisibility
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFieldVisibility {
    #[serde(alias = "PublicSafe")]
    PublicSafe,
    #[serde(alias = "OperatorOnly")]
    OperatorOnly,
    #[serde(alias = "ControllerOnly")]
    ControllerOnly,
    #[serde(alias = "FeatureGated")]
    FeatureGated,
    #[serde(alias = "Disabled")]
    Disabled,
}

///
/// RuntimeCheckStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCheckStatus {
    #[serde(alias = "Pass")]
    Pass,
    #[serde(alias = "Warn")]
    Warn,
    #[serde(alias = "Fail")]
    Fail,
    #[serde(alias = "NotEvaluated")]
    NotEvaluated,
}

///
/// RuntimeDiagnosticSeverity
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDiagnosticSeverity {
    #[serde(alias = "Info")]
    Info,
    #[serde(alias = "Warning")]
    Warning,
    #[serde(alias = "Blocked")]
    Blocked,
    #[serde(alias = "Unsupported")]
    Unsupported,
}

///
/// RuntimeStateDomainStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStateDomainStatus {
    #[serde(alias = "Ok")]
    Ok,
    #[serde(alias = "Warning")]
    Warning,
    #[serde(alias = "Failing")]
    Failing,
    #[serde(alias = "NotEvaluated")]
    NotEvaluated,
}
