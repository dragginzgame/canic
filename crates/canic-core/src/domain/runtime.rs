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

impl FailureSeverity {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
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

impl RuntimeFieldVisibility {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PublicSafe => "public_safe",
            Self::OperatorOnly => "operator_only",
            Self::ControllerOnly => "controller_only",
            Self::FeatureGated => "feature_gated",
            Self::Disabled => "disabled",
        }
    }
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

impl RuntimeCheckStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::NotEvaluated => "not_evaluated",
        }
    }
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

impl RuntimeDiagnosticSeverity {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Blocked => "blocked",
            Self::Unsupported => "unsupported",
        }
    }
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

impl RuntimeStateDomainStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Failing => "failing",
            Self::NotEvaluated => "not_evaluated",
        }
    }
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

impl HealthStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }
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

impl ReadinessStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::NotReady => "not_ready",
            Self::NotEvaluated => "not_evaluated",
        }
    }
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

impl RuntimeStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Failing => "failing",
            Self::Unknown => "unknown",
        }
    }
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

impl TimerStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Delayed => "delayed",
            Self::Failing => "failing",
            Self::Disabled => "disabled",
            Self::NotRegistered => "not_registered",
            Self::Unknown => "unknown",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_enums_own_serialized_labels() {
        assert_eq!(FailureSeverity::Info.label(), "info");
        assert_eq!(FailureSeverity::Warning.label(), "warning");
        assert_eq!(FailureSeverity::Error.label(), "error");
        assert_eq!(FailureSeverity::Critical.label(), "critical");

        assert_eq!(RuntimeFieldVisibility::PublicSafe.label(), "public_safe");
        assert_eq!(
            RuntimeFieldVisibility::OperatorOnly.label(),
            "operator_only"
        );
        assert_eq!(
            RuntimeFieldVisibility::ControllerOnly.label(),
            "controller_only"
        );
        assert_eq!(
            RuntimeFieldVisibility::FeatureGated.label(),
            "feature_gated"
        );
        assert_eq!(RuntimeFieldVisibility::Disabled.label(), "disabled");

        assert_eq!(RuntimeCheckStatus::Pass.label(), "pass");
        assert_eq!(RuntimeCheckStatus::Warn.label(), "warn");
        assert_eq!(RuntimeCheckStatus::Fail.label(), "fail");
        assert_eq!(RuntimeCheckStatus::NotEvaluated.label(), "not_evaluated");

        assert_eq!(RuntimeDiagnosticSeverity::Info.label(), "info");
        assert_eq!(RuntimeDiagnosticSeverity::Warning.label(), "warning");
        assert_eq!(RuntimeDiagnosticSeverity::Blocked.label(), "blocked");
        assert_eq!(
            RuntimeDiagnosticSeverity::Unsupported.label(),
            "unsupported"
        );

        assert_eq!(RuntimeStateDomainStatus::Ok.label(), "ok");
        assert_eq!(RuntimeStateDomainStatus::Warning.label(), "warning");
        assert_eq!(RuntimeStateDomainStatus::Failing.label(), "failing");
        assert_eq!(
            RuntimeStateDomainStatus::NotEvaluated.label(),
            "not_evaluated"
        );

        assert_eq!(HealthStatus::Healthy.label(), "healthy");
        assert_eq!(HealthStatus::Degraded.label(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.label(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.label(), "unknown");

        assert_eq!(ReadinessStatus::Ready.label(), "ready");
        assert_eq!(ReadinessStatus::Degraded.label(), "degraded");
        assert_eq!(ReadinessStatus::NotReady.label(), "not_ready");
        assert_eq!(ReadinessStatus::NotEvaluated.label(), "not_evaluated");

        assert_eq!(RuntimeStatus::Ok.label(), "ok");
        assert_eq!(RuntimeStatus::Degraded.label(), "degraded");
        assert_eq!(RuntimeStatus::Failing.label(), "failing");
        assert_eq!(RuntimeStatus::Unknown.label(), "unknown");

        assert_eq!(TimerStatus::Healthy.label(), "healthy");
        assert_eq!(TimerStatus::Delayed.label(), "delayed");
        assert_eq!(TimerStatus::Failing.label(), "failing");
        assert_eq!(TimerStatus::Disabled.label(), "disabled");
        assert_eq!(TimerStatus::NotRegistered.label(), "not_registered");
        assert_eq!(TimerStatus::Unknown.label(), "unknown");
    }
}
