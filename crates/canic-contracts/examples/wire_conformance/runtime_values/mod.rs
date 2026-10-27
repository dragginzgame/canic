//! Generated-client vectors for each renamed unit enum used by runtime diagnostics.
use canic_contracts::{deployment::ComponentChildKind, ids::BuildNetwork, values::runtime::*};
use ic_wire::{Limits, native, typescript};
use serde_json::{Value, json};

pub fn append(
    methods: &mut Vec<Value>,
    base: &Value,
    limits: Limits,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut vectors = Vec::new();
    append_diagnostics(methods, &mut vectors, base, limits)?;
    append_readiness(methods, &mut vectors, base, limits)?;
    append_artifact_values(methods, &mut vectors, base, limits)?;
    Ok(Value::Array(vectors))
}
fn append_diagnostics(
    methods: &mut Vec<Value>,
    vectors: &mut Vec<Value>,
    base: &Value,
    limits: Limits,
) -> Result<(), Box<dyn std::error::Error>> {
    append_enum(
        methods,
        vectors,
        base,
        22,
        "FailureSeverity",
        &[
            FailureSeverity::Info,
            FailureSeverity::Warning,
            FailureSeverity::Error,
            FailureSeverity::Critical,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        23,
        "RuntimeFieldVisibility",
        &[
            RuntimeFieldVisibility::PublicSafe,
            RuntimeFieldVisibility::OperatorOnly,
            RuntimeFieldVisibility::ControllerOnly,
            RuntimeFieldVisibility::FeatureGated,
            RuntimeFieldVisibility::Disabled,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        24,
        "RuntimeCheckStatus",
        &[
            RuntimeCheckStatus::Pass,
            RuntimeCheckStatus::Warn,
            RuntimeCheckStatus::Fail,
            RuntimeCheckStatus::NotEvaluated,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        25,
        "RuntimeDiagnosticSeverity",
        &[
            RuntimeDiagnosticSeverity::Info,
            RuntimeDiagnosticSeverity::Warning,
            RuntimeDiagnosticSeverity::Blocked,
            RuntimeDiagnosticSeverity::Unsupported,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        26,
        "RuntimeStateDomainStatus",
        &[
            RuntimeStateDomainStatus::Ok,
            RuntimeStateDomainStatus::Warning,
            RuntimeStateDomainStatus::Failing,
            RuntimeStateDomainStatus::NotEvaluated,
        ],
        limits,
    )?;
    Ok(())
}
fn append_readiness(
    methods: &mut Vec<Value>,
    vectors: &mut Vec<Value>,
    base: &Value,
    limits: Limits,
) -> Result<(), Box<dyn std::error::Error>> {
    append_enum(
        methods,
        vectors,
        base,
        27,
        "HealthStatus",
        &[
            HealthStatus::Healthy,
            HealthStatus::Degraded,
            HealthStatus::Unhealthy,
            HealthStatus::Unknown,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        28,
        "ReadinessStatus",
        &[
            ReadinessStatus::Ready,
            ReadinessStatus::Degraded,
            ReadinessStatus::NotReady,
            ReadinessStatus::NotEvaluated,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        29,
        "RuntimeStatus",
        &[
            RuntimeStatus::Ok,
            RuntimeStatus::Degraded,
            RuntimeStatus::Failing,
            RuntimeStatus::Unknown,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        30,
        "TimerRegistrationStatus",
        &[
            TimerRegistrationStatus::Unregistered,
            TimerRegistrationStatus::Scheduled,
            TimerRegistrationStatus::Running,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        31,
        "TimerProcessCondition",
        &[
            TimerProcessCondition::Disabled,
            TimerProcessCondition::Idle,
            TimerProcessCondition::Active,
            TimerProcessCondition::Retrying,
            TimerProcessCondition::Failed,
            TimerProcessCondition::MissingRegistration,
        ],
        limits,
    )?;
    Ok(())
}
fn append_artifact_values(
    methods: &mut Vec<Value>,
    vectors: &mut Vec<Value>,
    base: &Value,
    limits: Limits,
) -> Result<(), Box<dyn std::error::Error>> {
    append_enum(
        methods,
        vectors,
        base,
        32,
        "TimerSchedulingMode",
        &[
            TimerSchedulingMode::Once,
            TimerSchedulingMode::AfterCompletion,
            TimerSchedulingMode::Deadline,
            TimerSchedulingMode::Retry,
            TimerSchedulingMode::Continuation,
            TimerSchedulingMode::Watchdog,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        33,
        "TimerExecutionOutcome",
        &[
            TimerExecutionOutcome::Success,
            TimerExecutionOutcome::NoWork,
            TimerExecutionOutcome::RetryableFailure,
            TimerExecutionOutcome::InvariantFailure,
            TimerExecutionOutcome::Unacknowledged,
        ],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        34,
        "BuildNetwork",
        &[BuildNetwork::Ic, BuildNetwork::Local],
        limits,
    )?;
    append_enum(
        methods,
        vectors,
        base,
        35,
        "ComponentChildKind",
        &[
            ComponentChildKind::Singleton,
            ComponentChildKind::Replica,
            ComponentChildKind::Shard,
            ComponentChildKind::Instance,
        ],
        limits,
    )?;
    Ok(())
}

fn append_enum<T: typescript::Wire>(
    methods: &mut Vec<Value>,
    vectors: &mut Vec<Value>,
    base: &Value,
    id: u64,
    name: &str,
    values: &[T],
    limits: Limits,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut method = base.clone();
    method["id"] = json!(id.to_string());
    method["name"] = json!(name);
    let schema = typescript::native_schema::<T>().map_err(|e| format!("enum schema: {e:?}"))?;
    method["request"] = schema.clone();
    method["response"] = schema;
    methods.push(method);
    for value in values {
        vectors.push(json!({
            "method":name, "label":serde_json::to_value(value)?,
            "request":native::encode_request(id, value, limits)?,
            "response":native::encode_response(id, value, limits)?,
        }));
    }
    Ok(())
}
