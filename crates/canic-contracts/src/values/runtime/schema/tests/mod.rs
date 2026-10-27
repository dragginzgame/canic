//! Exact native tag and configuration label vectors.
use super::*;
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 1024,
    nodes: 32,
    depth: 8,
};

#[test]
fn failure_severity_labels_and_tags_match_serde() {
    let values = [
        FailureSeverity::Info,
        FailureSeverity::Warning,
        FailureSeverity::Error,
        FailureSeverity::Critical,
    ];
    let schema = ic_wire::typescript::native_schema::<FailureSeverity>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            FailureSeverity::Info => 0usize,
            FailureSeverity::Warning => 1usize,
            FailureSeverity::Error => 2usize,
            FailureSeverity::Critical => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<FailureSeverity>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn runtime_field_visibility_labels_and_tags_match_serde() {
    let values = [
        RuntimeFieldVisibility::PublicSafe,
        RuntimeFieldVisibility::OperatorOnly,
        RuntimeFieldVisibility::ControllerOnly,
        RuntimeFieldVisibility::FeatureGated,
        RuntimeFieldVisibility::Disabled,
    ];
    let schema = ic_wire::typescript::native_schema::<RuntimeFieldVisibility>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            RuntimeFieldVisibility::PublicSafe => 0usize,
            RuntimeFieldVisibility::OperatorOnly => 1usize,
            RuntimeFieldVisibility::ControllerOnly => 2usize,
            RuntimeFieldVisibility::FeatureGated => 3usize,
            RuntimeFieldVisibility::Disabled => 4usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<RuntimeFieldVisibility>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn runtime_check_status_labels_and_tags_match_serde() {
    let values = [
        RuntimeCheckStatus::Pass,
        RuntimeCheckStatus::Warn,
        RuntimeCheckStatus::Fail,
        RuntimeCheckStatus::NotEvaluated,
    ];
    let schema = ic_wire::typescript::native_schema::<RuntimeCheckStatus>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            RuntimeCheckStatus::Pass => 0usize,
            RuntimeCheckStatus::Warn => 1usize,
            RuntimeCheckStatus::Fail => 2usize,
            RuntimeCheckStatus::NotEvaluated => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<RuntimeCheckStatus>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn runtime_diagnostic_severity_labels_and_tags_match_serde() {
    let values = [
        RuntimeDiagnosticSeverity::Info,
        RuntimeDiagnosticSeverity::Warning,
        RuntimeDiagnosticSeverity::Blocked,
        RuntimeDiagnosticSeverity::Unsupported,
    ];
    let schema = ic_wire::typescript::native_schema::<RuntimeDiagnosticSeverity>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            RuntimeDiagnosticSeverity::Info => 0usize,
            RuntimeDiagnosticSeverity::Warning => 1usize,
            RuntimeDiagnosticSeverity::Blocked => 2usize,
            RuntimeDiagnosticSeverity::Unsupported => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<RuntimeDiagnosticSeverity>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn runtime_state_domain_status_labels_and_tags_match_serde() {
    let values = [
        RuntimeStateDomainStatus::Ok,
        RuntimeStateDomainStatus::Warning,
        RuntimeStateDomainStatus::Failing,
        RuntimeStateDomainStatus::NotEvaluated,
    ];
    let schema = ic_wire::typescript::native_schema::<RuntimeStateDomainStatus>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            RuntimeStateDomainStatus::Ok => 0usize,
            RuntimeStateDomainStatus::Warning => 1usize,
            RuntimeStateDomainStatus::Failing => 2usize,
            RuntimeStateDomainStatus::NotEvaluated => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<RuntimeStateDomainStatus>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn health_status_labels_and_tags_match_serde() {
    let values = [
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Unhealthy,
        HealthStatus::Unknown,
    ];
    let schema = ic_wire::typescript::native_schema::<HealthStatus>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            HealthStatus::Healthy => 0usize,
            HealthStatus::Degraded => 1usize,
            HealthStatus::Unhealthy => 2usize,
            HealthStatus::Unknown => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<HealthStatus>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn readiness_status_labels_and_tags_match_serde() {
    let values = [
        ReadinessStatus::Ready,
        ReadinessStatus::Degraded,
        ReadinessStatus::NotReady,
        ReadinessStatus::NotEvaluated,
    ];
    let schema = ic_wire::typescript::native_schema::<ReadinessStatus>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            ReadinessStatus::Ready => 0usize,
            ReadinessStatus::Degraded => 1usize,
            ReadinessStatus::NotReady => 2usize,
            ReadinessStatus::NotEvaluated => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<ReadinessStatus>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn runtime_status_labels_and_tags_match_serde() {
    let values = [
        RuntimeStatus::Ok,
        RuntimeStatus::Degraded,
        RuntimeStatus::Failing,
        RuntimeStatus::Unknown,
    ];
    let schema = ic_wire::typescript::native_schema::<RuntimeStatus>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            RuntimeStatus::Ok => 0usize,
            RuntimeStatus::Degraded => 1usize,
            RuntimeStatus::Failing => 2usize,
            RuntimeStatus::Unknown => 3usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<RuntimeStatus>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn timer_registration_status_labels_and_tags_match_serde() {
    let values = [
        TimerRegistrationStatus::Unregistered,
        TimerRegistrationStatus::Scheduled,
        TimerRegistrationStatus::Running,
    ];
    let schema = ic_wire::typescript::native_schema::<TimerRegistrationStatus>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            TimerRegistrationStatus::Unregistered => 0usize,
            TimerRegistrationStatus::Scheduled => 1usize,
            TimerRegistrationStatus::Running => 2usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<TimerRegistrationStatus>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn timer_process_condition_labels_and_tags_match_serde() {
    let values = [
        TimerProcessCondition::Disabled,
        TimerProcessCondition::Idle,
        TimerProcessCondition::Active,
        TimerProcessCondition::Retrying,
        TimerProcessCondition::Failed,
        TimerProcessCondition::MissingRegistration,
    ];
    let schema = ic_wire::typescript::native_schema::<TimerProcessCondition>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            TimerProcessCondition::Disabled => 0usize,
            TimerProcessCondition::Idle => 1usize,
            TimerProcessCondition::Active => 2usize,
            TimerProcessCondition::Retrying => 3usize,
            TimerProcessCondition::Failed => 4usize,
            TimerProcessCondition::MissingRegistration => 5usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<TimerProcessCondition>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn timer_scheduling_mode_labels_and_tags_match_serde() {
    let values = [
        TimerSchedulingMode::Once,
        TimerSchedulingMode::AfterCompletion,
        TimerSchedulingMode::Deadline,
        TimerSchedulingMode::Retry,
        TimerSchedulingMode::Continuation,
        TimerSchedulingMode::Watchdog,
    ];
    let schema = ic_wire::typescript::native_schema::<TimerSchedulingMode>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            TimerSchedulingMode::Once => 0usize,
            TimerSchedulingMode::AfterCompletion => 1usize,
            TimerSchedulingMode::Deadline => 2usize,
            TimerSchedulingMode::Retry => 3usize,
            TimerSchedulingMode::Continuation => 4usize,
            TimerSchedulingMode::Watchdog => 5usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<TimerSchedulingMode>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}

#[test]
fn timer_execution_outcome_labels_and_tags_match_serde() {
    let values = [
        TimerExecutionOutcome::Success,
        TimerExecutionOutcome::NoWork,
        TimerExecutionOutcome::RetryableFailure,
        TimerExecutionOutcome::InvariantFailure,
        TimerExecutionOutcome::Unacknowledged,
    ];
    let schema = ic_wire::typescript::native_schema::<TimerExecutionOutcome>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            TimerExecutionOutcome::Success => 0usize,
            TimerExecutionOutcome::NoWork => 1usize,
            TimerExecutionOutcome::RetryableFailure => 2usize,
            TimerExecutionOutcome::InvariantFailure => 3usize,
            TimerExecutionOutcome::Unacknowledged => 4usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<TimerExecutionOutcome>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}
