use crate::{dto::prelude::*, ids::BuildNetwork};

pub use crate::values::runtime::{
    FailureSeverity, HealthStatus, ReadinessStatus, RuntimeCheckStatus, RuntimeDiagnosticSeverity,
    RuntimeFieldVisibility, RuntimeStateDomainStatus, RuntimeStatus, TimerExecutionOutcome,
    TimerProcessCondition, TimerRegistrationStatus, TimerSchedulingMode,
};

pub const RUNTIME_INTROSPECTION_SCHEMA_VERSION: u32 = 1;

//
// CanicHealthStatus
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanicHealthStatus {
    pub schema_version: u32,
    pub status: HealthStatus,
    pub observed_at_ns: Option<u64>,
    pub checks: Vec<RuntimeCheck>,
}

//
// CanicReadinessStatus
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicReadinessStatus {
    pub fixture: Result<
        crate::dto::fixture_provisioning::FixtureProvisioningStatus,
        crate::dto::fixture_provisioning::FixtureImportError,
    >,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanicRuntimeStatus {
    pub schema_version: u32,
    pub observed_at_ns: u64,
    pub canister_id: Principal,
    pub role: Option<String>,
    pub root: Option<Principal>,
    pub build_network: Option<BuildNetwork>,
    pub build: RuntimeBuildInfo,
    pub features: Vec<RuntimeFeatureStatus>,
    pub topology: Option<RuntimeTopologyStatus>,
    pub timers: Vec<CanisterTimerStatus>,
    pub timer_inventory: RuntimeCheck,
    pub state: Option<RuntimeStateSummary>,
    pub auth: Option<RuntimeAuthStatusSummary>,
    pub receipt_capacity: Option<RuntimeReceiptCapacityStatus>,
    pub recent_failures: Vec<RecentFailure>,
    pub visibility: Vec<RuntimeVisibilityEntry>,
    pub readiness: CanicReadinessStatus,
    pub status: RuntimeStatus,
}

//
// RuntimeBuildInfo
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeBuildInfo {
    pub package_name: String,
    pub package_version: String,
    pub canic_version: String,
    pub canister_version: u64,
}

//
// RuntimeCheck
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeFeatureStatus {
    pub name: String,
    pub enabled: bool,
    pub visibility: RuntimeFieldVisibility,
    pub source: String,
}

//
// RuntimeTopologyStatus
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeTopologyStatus {
    pub root: Option<Principal>,
    pub parent: Option<Principal>,
    pub subnet: Option<Principal>,
    pub source: String,
}

//
// CanisterTimerStatus
//

/// One live or retained registration from the shared canister timer registry.
///
/// A transient `RemoveWhenStopped` registration is absent after reaching its
/// terminal state; its final observation is therefore not retained here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanisterTimerStatus {
    pub name: String,
    pub owner: String,
    pub subsystem: String,
    pub scheduling_mode: TimerSchedulingMode,
    pub registration: TimerRegistrationStatus,
    pub condition: TimerProcessCondition,
    pub enabled: bool,
    pub generation: Option<u64>,
    pub next_due_at_ns: Option<u64>,
    pub last_outcome: Option<TimerExecutionOutcome>,
    pub last_work_count: u64,
    pub last_success_at_ns: Option<u64>,
    pub last_failure_at_ns: Option<u64>,
    pub consecutive_expected_failures: u64,
    pub schedules_since_runtime_start: u64,
    pub executions_since_runtime_start: u64,
    pub successes_since_runtime_start: u64,
    pub expected_failures_since_runtime_start: u64,
    pub invariant_failures_since_runtime_start: u64,
    pub stale_callbacks_since_runtime_start: u64,
    pub scheduler_performance: TimerCallbackPerformanceStatus,
    pub work_performance: TimerCallbackPerformanceStatus,
}

//
// TimerCallbackPerformanceStatus
//

/// Bounded, runtime-epoch observations for one timer callback role.
///
/// Samples cover the complete accepted shared-runtime callback path rather
/// than isolating the consumer function from registry and completion work.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TimerCallbackPerformanceStatus {
    pub instruction_samples_since_runtime_start: u64,
    pub instructions_latest: Option<u64>,
    pub instructions_maximum: Option<u64>,
    pub instructions_total_since_runtime_start: u128,
    pub memory_page_samples_since_runtime_start: u64,
    pub memory_pages_latest: Option<TimerMemoryPageSampleStatus>,
    pub maximum_wasm_memory_growth_pages: Option<u64>,
    pub maximum_stable_memory_growth_pages: Option<u64>,
}

//
// TimerMemoryPageSampleStatus
//

/// Wasm and stable-memory page extents bracketing one completed callback.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TimerMemoryPageSampleStatus {
    pub start: TimerMemoryPageExtentStatus,
    pub end: TimerMemoryPageExtentStatus,
}

//
// TimerMemoryPageExtentStatus
//

/// Runtime high-water memory extents measured in 64-KiB pages.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TimerMemoryPageExtentStatus {
    pub wasm_pages: u64,
    pub stable_pages: u64,
}

//
// RuntimeStateSummary
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeStateSummary {
    pub manifest_schema_version: u32,
    pub domains: Vec<RuntimeStateDomainSummary>,
    pub total_stable_memory_pages: Option<u64>,
}

//
// RuntimeStateDomainSummary
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeAuthStatusSummary {
    pub auth_features: Vec<RuntimeFeatureStatus>,
}

//
// RuntimeReceiptCapacityStatus
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeReceiptCapacityStatus {
    pub status: RuntimeCheckStatus,
    pub receipt_records: u64,
    pub application_receipt_records: u64,
    pub canic_owned_receipt_records: u64,
    pub pending_application_receipt_records: u64,
    pub terminal_application_receipt_records: u64,
    pub receipt_record_limit: u64,
    pub remaining_receipt_record_headroom: u64,
    pub resource_total_records: u64,
    pub resource_total_record_limit: u64,
    pub remaining_resource_total_headroom: u64,
    pub warning_headroom_threshold: u64,
    pub reserved_terminal_slots: u64,
    pub reserved_terminal_pages: u64,
    pub next_terminal_eligibility_at_ns: Option<u64>,
    pub source: String,
}

//
// RecentFailure
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RuntimeVisibilityEntry {
    pub field: String,
    pub visibility: RuntimeFieldVisibility,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    #[test]
    fn timer_status_retains_instruction_totals_above_u64() {
        let status = TimerCallbackPerformanceStatus {
            instruction_samples_since_runtime_start: 2,
            instructions_latest: Some(1),
            instructions_maximum: Some(u64::MAX),
            instructions_total_since_runtime_start: u128::from(u64::MAX) + 1,
            memory_page_samples_since_runtime_start: 0,
            memory_pages_latest: None,
            maximum_wasm_memory_growth_pages: None,
            maximum_stable_memory_growth_pages: None,
        };
        // A scalar-contract fixture exercises this nested value's actual Wire encoding.
        let method = 1;
        let limits = crate::protocol::protected_runtime::RESPONSE_LIMITS;
        let bytes = ic_wire::native::encode_response(method, &status, limits).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<TimerCallbackPerformanceStatus>(
                method, &bytes, limits
            )
            .unwrap(),
            status
        );
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            serde_json::from_str::<TimerCallbackPerformanceStatus>(&json).unwrap(),
            status
        );
    }

    #[test]
    fn runtime_enums_serialize_canonical_snake_case_labels() {
        assert_enum_serde_contract(HealthStatus::Healthy, HealthStatus::Healthy.label());
        assert_enum_serde_contract(HealthStatus::Degraded, HealthStatus::Degraded.label());
        assert_enum_serde_contract(HealthStatus::Unhealthy, HealthStatus::Unhealthy.label());
        assert_enum_serde_contract(HealthStatus::Unknown, HealthStatus::Unknown.label());

        assert_enum_serde_contract(ReadinessStatus::Ready, ReadinessStatus::Ready.label());
        assert_enum_serde_contract(ReadinessStatus::Degraded, ReadinessStatus::Degraded.label());
        assert_enum_serde_contract(ReadinessStatus::NotReady, ReadinessStatus::NotReady.label());
        assert_enum_serde_contract(
            ReadinessStatus::NotEvaluated,
            ReadinessStatus::NotEvaluated.label(),
        );

        assert_enum_serde_contract(RuntimeStatus::Ok, RuntimeStatus::Ok.label());
        assert_enum_serde_contract(RuntimeStatus::Degraded, RuntimeStatus::Degraded.label());
        assert_enum_serde_contract(RuntimeStatus::Failing, RuntimeStatus::Failing.label());
        assert_enum_serde_contract(RuntimeStatus::Unknown, RuntimeStatus::Unknown.label());

        assert_enum_serde_contract(
            TimerRegistrationStatus::Unregistered,
            TimerRegistrationStatus::Unregistered.label(),
        );
        assert_enum_serde_contract(
            TimerProcessCondition::MissingRegistration,
            TimerProcessCondition::MissingRegistration.label(),
        );
        assert_enum_serde_contract(
            TimerSchedulingMode::AfterCompletion,
            TimerSchedulingMode::AfterCompletion.label(),
        );
        assert_enum_serde_contract(
            TimerExecutionOutcome::InvariantFailure,
            TimerExecutionOutcome::InvariantFailure.label(),
        );

        assert_enum_serde_contract(FailureSeverity::Info, FailureSeverity::Info.label());
        assert_enum_serde_contract(FailureSeverity::Warning, FailureSeverity::Warning.label());
        assert_enum_serde_contract(FailureSeverity::Error, FailureSeverity::Error.label());
        assert_enum_serde_contract(FailureSeverity::Critical, FailureSeverity::Critical.label());

        assert_enum_serde_contract(RuntimeCheckStatus::Pass, RuntimeCheckStatus::Pass.label());
        assert_enum_serde_contract(RuntimeCheckStatus::Warn, RuntimeCheckStatus::Warn.label());
        assert_enum_serde_contract(RuntimeCheckStatus::Fail, RuntimeCheckStatus::Fail.label());
        assert_enum_serde_contract(
            RuntimeCheckStatus::NotEvaluated,
            RuntimeCheckStatus::NotEvaluated.label(),
        );

        assert_enum_serde_contract(
            RuntimeDiagnosticSeverity::Info,
            RuntimeDiagnosticSeverity::Info.label(),
        );
        assert_enum_serde_contract(
            RuntimeDiagnosticSeverity::Warning,
            RuntimeDiagnosticSeverity::Warning.label(),
        );
        assert_enum_serde_contract(
            RuntimeDiagnosticSeverity::Blocked,
            RuntimeDiagnosticSeverity::Blocked.label(),
        );
        assert_enum_serde_contract(
            RuntimeDiagnosticSeverity::Unsupported,
            RuntimeDiagnosticSeverity::Unsupported.label(),
        );

        assert_enum_serde_contract(
            RuntimeFieldVisibility::PublicSafe,
            RuntimeFieldVisibility::PublicSafe.label(),
        );
        assert_enum_serde_contract(
            RuntimeFieldVisibility::OperatorOnly,
            RuntimeFieldVisibility::OperatorOnly.label(),
        );
        assert_enum_serde_contract(
            RuntimeFieldVisibility::ControllerOnly,
            RuntimeFieldVisibility::ControllerOnly.label(),
        );
        assert_enum_serde_contract(
            RuntimeFieldVisibility::FeatureGated,
            RuntimeFieldVisibility::FeatureGated.label(),
        );
        assert_enum_serde_contract(
            RuntimeFieldVisibility::Disabled,
            RuntimeFieldVisibility::Disabled.label(),
        );

        assert_enum_serde_contract(
            RuntimeStateDomainStatus::Ok,
            RuntimeStateDomainStatus::Ok.label(),
        );
        assert_enum_serde_contract(
            RuntimeStateDomainStatus::Warning,
            RuntimeStateDomainStatus::Warning.label(),
        );
        assert_enum_serde_contract(
            RuntimeStateDomainStatus::Failing,
            RuntimeStateDomainStatus::Failing.label(),
        );
        assert_enum_serde_contract(
            RuntimeStateDomainStatus::NotEvaluated,
            RuntimeStateDomainStatus::NotEvaluated.label(),
        );
    }

    fn assert_enum_serde_contract<T>(value: T, label: &str)
    where
        T: Clone + Debug + DeserializeOwned + Eq + Serialize,
    {
        let encoded = crate::serialization::serialize(&value).expect("encode runtime enum label");
        let serialized_label: String =
            crate::serialization::deserialize(&encoded).expect("decode runtime enum label");
        assert_eq!(serialized_label, label);

        let label_bytes =
            crate::serialization::serialize(&label).expect("encode canonical runtime enum label");
        let decoded: T = crate::serialization::deserialize(&label_bytes)
            .expect("decode canonical runtime enum label");
        assert_eq!(decoded, value);
    }
}
