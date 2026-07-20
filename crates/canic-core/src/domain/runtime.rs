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
/// TimerRegistrationStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerRegistrationStatus {
    #[serde(rename = "unregistered")]
    Unregistered,
    #[serde(rename = "scheduled")]
    Scheduled,
    #[serde(rename = "running")]
    Running,
}

impl TimerRegistrationStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unregistered => "unregistered",
            Self::Scheduled => "scheduled",
            Self::Running => "running",
        }
    }
}

///
/// TimerProcessCondition
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerProcessCondition {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "retrying")]
    Retrying,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "missing_registration")]
    MissingRegistration,
}

impl TimerProcessCondition {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Idle => "idle",
            Self::Active => "active",
            Self::Retrying => "retrying",
            Self::Failed => "failed",
            Self::MissingRegistration => "missing_registration",
        }
    }
}

///
/// TimerSchedulingMode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerSchedulingMode {
    #[serde(rename = "once")]
    Once,
    #[serde(rename = "after_completion")]
    AfterCompletion,
    #[serde(rename = "deadline")]
    Deadline,
    #[serde(rename = "retry")]
    Retry,
    #[serde(rename = "continuation")]
    Continuation,
}

impl TimerSchedulingMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::AfterCompletion => "after_completion",
            Self::Deadline => "deadline",
            Self::Retry => "retry",
            Self::Continuation => "continuation",
        }
    }
}

///
/// TimerExecutionOutcome
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerExecutionOutcome {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "no_work")]
    NoWork,
    #[serde(rename = "retryable_failure")]
    RetryableFailure,
    #[serde(rename = "invariant_failure")]
    InvariantFailure,
}

impl TimerExecutionOutcome {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::NoWork => "no_work",
            Self::RetryableFailure => "retryable_failure",
            Self::InvariantFailure => "invariant_failure",
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

        assert_eq!(
            TimerRegistrationStatus::Unregistered.label(),
            "unregistered"
        );
        assert_eq!(TimerRegistrationStatus::Scheduled.label(), "scheduled");
        assert_eq!(TimerRegistrationStatus::Running.label(), "running");

        assert_eq!(TimerProcessCondition::Disabled.label(), "disabled");
        assert_eq!(TimerProcessCondition::Idle.label(), "idle");
        assert_eq!(TimerProcessCondition::Active.label(), "active");
        assert_eq!(TimerProcessCondition::Retrying.label(), "retrying");
        assert_eq!(TimerProcessCondition::Failed.label(), "failed");
        assert_eq!(
            TimerProcessCondition::MissingRegistration.label(),
            "missing_registration"
        );

        assert_eq!(TimerSchedulingMode::Once.label(), "once");
        assert_eq!(
            TimerSchedulingMode::AfterCompletion.label(),
            "after_completion"
        );
        assert_eq!(TimerSchedulingMode::Deadline.label(), "deadline");
        assert_eq!(TimerSchedulingMode::Retry.label(), "retry");
        assert_eq!(TimerSchedulingMode::Continuation.label(), "continuation");

        assert_eq!(TimerExecutionOutcome::Success.label(), "success");
        assert_eq!(TimerExecutionOutcome::NoWork.label(), "no_work");
        assert_eq!(
            TimerExecutionOutcome::RetryableFailure.label(),
            "retryable_failure"
        );
        assert_eq!(
            TimerExecutionOutcome::InvariantFailure.label(),
            "invariant_failure"
        );
    }
}
