pub mod install;

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::runtime::{
        FailureSeverity, HealthStatus, ReadinessStatus, RuntimeCheckStatus,
        RuntimeDiagnosticSeverity, RuntimeFieldVisibility, RuntimeStateDomainStatus, RuntimeStatus,
    },
    dto::{
        error::Error,
        runtime::{
            CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, CanisterTimerStatus,
            RUNTIME_INTROSPECTION_SCHEMA_VERSION, RuntimeAuthStatusSummary,
            RuntimeBlobStorageStatusSummary, RuntimeBuildInfo, RuntimeCheck, RuntimeDiagnostic,
            RuntimeFeatureStatus, RuntimeReceiptCapacityStatus, RuntimeStateDomainSummary,
            RuntimeStateSummary, RuntimeTopologyStatus, RuntimeVisibilityEntry,
            TimerCallbackPerformanceStatus, TimerMemoryPageExtentStatus,
            TimerMemoryPageSampleStatus,
        },
    },
    ops::{
        ic::{IcOps, build_network::BuildNetworkOps},
        runtime::{
            env::EnvOps,
            memory::MemoryRegistryOps,
            ready::ReadyOps,
            recent_failure::{RecentFailureInput, RecentFailureOps},
        },
        storage::intent::{RECEIPT_CAPACITY_WARNING_HEADROOM_THRESHOLD, ReceiptBackedIntentOps},
    },
    state_contract::{STATE_MANIFEST_SCHEMA_VERSION, canic_state_descriptors},
    workflow::runtime::timer::TimerWorkflow,
};
const RUNTIME_FEATURE_SOURCE: &str = "compile_feature";
const RUNTIME_FEATURE_FLAGS: [(&str, bool); 10] = [
    (
        "auth-chain-key-ecdsa",
        cfg!(feature = "auth-chain-key-ecdsa"),
    ),
    (
        "auth-chain-key-root-sign",
        cfg!(feature = "auth-chain-key-root-sign"),
    ),
    (
        "auth-delegated-token-verify",
        cfg!(feature = "auth-delegated-token-verify"),
    ),
    (
        "auth-issuer-canister-sig-create",
        cfg!(feature = "auth-issuer-canister-sig-create"),
    ),
    (
        "auth-issuer-canister-sig-verify",
        cfg!(feature = "auth-issuer-canister-sig-verify"),
    ),
    (
        "auth-root-canister-sig-create",
        cfg!(feature = "auth-root-canister-sig-create"),
    ),
    (
        "auth-root-canister-sig-verify",
        cfg!(feature = "auth-root-canister-sig-verify"),
    ),
    ("blob-storage", cfg!(feature = "blob-storage")),
    (
        "blob-storage-billing",
        cfg!(feature = "blob-storage-billing"),
    ),
    ("sharding", cfg!(feature = "sharding")),
];

///
/// MemoryRuntimeApi
///

pub struct MemoryRuntimeApi;

impl MemoryRuntimeApi {
    /// Bootstrap Canic's stable-memory declaration snapshot.
    pub fn bootstrap_registry() -> Result<(), Error> {
        MemoryRegistryOps::bootstrap_registry().map_err(Error::from)?;

        Ok(())
    }
}

///
/// RuntimeIntrospectionApi
///

pub struct RuntimeIntrospectionApi;

impl RuntimeIntrospectionApi {
    /// Return the minimal health status for a canister that answered the query.
    #[must_use]
    pub fn health(observed_at_ns: Option<u64>) -> CanicHealthStatus {
        CanicHealthStatus {
            schema_version: RUNTIME_INTROSPECTION_SCHEMA_VERSION,
            status: HealthStatus::Healthy,
            observed_at_ns,
            checks: vec![RuntimeCheck {
                category: "health".to_string(),
                code: "canister_responsive".to_string(),
                status: RuntimeCheckStatus::Pass,
                subject: "canister".to_string(),
                detail: "canister returned a health response".to_string(),
                next: None,
                source: "runtime_observed".to_string(),
            }],
        }
    }

    /// Return guarded readiness status for the local Canic role.
    #[must_use]
    pub fn readiness(observed_at_ns: u64) -> CanicReadinessStatus {
        let ready = ReadyOps::is_ready();
        let role = EnvOps::canister_role()
            .ok()
            .map(crate::ids::CanisterRole::into_string);

        let (status, check_status, detail, next) = if ready {
            (
                ReadinessStatus::Ready,
                RuntimeCheckStatus::Pass,
                "runtime readiness barrier is marked ready",
                None,
            )
        } else {
            (
                ReadinessStatus::NotReady,
                RuntimeCheckStatus::Fail,
                "runtime readiness barrier is not ready",
                Some("wait for bootstrap to complete or inspect canic_bootstrap_status"),
            )
        };

        let readiness_check = RuntimeCheck {
            category: "readiness".to_string(),
            code: "runtime_ready_barrier".to_string(),
            status: check_status,
            subject: role.clone().unwrap_or_else(|| "unknown_role".to_string()),
            detail: detail.to_string(),
            next: next.map(str::to_string),
            source: "runtime_observed".to_string(),
        };

        let blockers = if ready {
            Vec::new()
        } else {
            vec![RuntimeDiagnostic {
                category: "readiness".to_string(),
                code: "runtime_not_ready".to_string(),
                severity: RuntimeDiagnosticSeverity::Blocked,
                subject: role.clone().unwrap_or_else(|| "unknown_role".to_string()),
                detail: "runtime readiness barrier has not completed".to_string(),
                next: Some(
                    "inspect bootstrap status before treating the role as ready".to_string(),
                ),
                source: "runtime_observed".to_string(),
            }]
        };

        CanicReadinessStatus {
            schema_version: RUNTIME_INTROSPECTION_SCHEMA_VERSION,
            role,
            status,
            observed_at_ns,
            checks: vec![readiness_check],
            blockers,
            warnings: Vec::new(),
        }
    }

    /// Return guarded runtime status for the local Canic role.
    #[must_use]
    pub fn runtime_status_for(
        canister_id: Principal,
        observed_at_ns: u64,
        package_name: &str,
        package_version: &str,
        canic_version: &str,
        canister_version: u64,
    ) -> CanicRuntimeStatus {
        let readiness = Self::readiness(observed_at_ns);
        let role = readiness.role.clone();
        let state = state_summary(role.as_deref());
        let root = EnvOps::root_pid().ok();
        let parent = EnvOps::parent_pid().ok();
        let subnet = EnvOps::subnet_pid().ok();
        let receipt_capacity_result = runtime_receipt_capacity();
        let receipt_capacity_status = receipt_capacity_result
            .as_ref()
            .ok()
            .map(|capacity| capacity.status);
        let timer_observation = timer_status_observation(TimerWorkflow::statuses(), observed_at_ns);
        let status = aggregate_runtime_status(
            readiness.status,
            receipt_capacity_status,
            timer_observation.check.status,
        );
        let (receipt_capacity, receipt_failure) = match receipt_capacity_result {
            Ok(capacity) => (Some(capacity), None),
            Err(err) => {
                let code = err.code();
                (
                    None,
                    Some(RecentFailureInput {
                        occurred_at_ns: observed_at_ns,
                        subsystem: "intent_capacity".to_string(),
                        code: code.to_string(),
                        severity: FailureSeverity::Error,
                        summary: format!("diagnostic={code}"),
                        correlation_id: None,
                    }),
                )
            }
        };
        let recent_failures = RecentFailureOps::snapshot_with_many(
            [timer_observation.failure, receipt_failure]
                .into_iter()
                .flatten(),
        );

        CanicRuntimeStatus {
            schema_version: RUNTIME_INTROSPECTION_SCHEMA_VERSION,
            observed_at_ns,
            canister_id,
            role,
            root,
            build_network: BuildNetworkOps::build_network(),
            build: RuntimeBuildInfo {
                package_name: package_name.to_string(),
                package_version: package_version.to_string(),
                canic_version: canic_version.to_string(),
                canister_version,
            },
            features: runtime_features(),
            topology: Some(RuntimeTopologyStatus {
                root,
                parent,
                subnet,
                source: "runtime_observed".to_string(),
            }),
            timers: timer_observation.timers,
            timer_inventory: timer_observation.check,
            state,
            auth: Some(runtime_auth_status()),
            blob_storage: runtime_blob_storage_status(),
            receipt_capacity,
            recent_failures,
            visibility: runtime_visibility(),
            readiness,
            status,
        }
    }

    /// Return guarded runtime status using ambient IC runtime values.
    #[must_use]
    pub fn runtime_status(
        observed_at_ns: u64,
        package_name: &str,
        package_version: &str,
        canic_version: &str,
        canister_version: u64,
    ) -> CanicRuntimeStatus {
        Self::runtime_status_for(
            IcOps::canister_self(),
            observed_at_ns,
            package_name,
            package_version,
            canic_version,
            canister_version,
        )
    }
}

fn runtime_features() -> Vec<RuntimeFeatureStatus> {
    RUNTIME_FEATURE_FLAGS
        .into_iter()
        .map(|(name, enabled)| runtime_feature_status(name, enabled))
        .collect()
}

fn runtime_feature_status(name: &str, enabled: bool) -> RuntimeFeatureStatus {
    RuntimeFeatureStatus {
        name: name.to_string(),
        enabled,
        visibility: RuntimeFieldVisibility::OperatorOnly,
        source: RUNTIME_FEATURE_SOURCE.to_string(),
    }
}

fn runtime_auth_status() -> RuntimeAuthStatusSummary {
    RuntimeAuthStatusSummary {
        auth_features: RUNTIME_FEATURE_FLAGS
            .into_iter()
            .filter(|(name, _)| name.starts_with("auth-"))
            .map(|(name, enabled)| runtime_feature_status(name, enabled))
            .collect(),
    }
}

fn runtime_receipt_capacity() -> Result<RuntimeReceiptCapacityStatus, InternalError> {
    let capacity = ReceiptBackedIntentOps::receipt_capacity()?;
    Ok(RuntimeReceiptCapacityStatus {
        status: receipt_capacity_condition(
            capacity.remaining_record_headroom,
            capacity.remaining_resource_total_headroom,
        ),
        receipt_records: capacity.total_records,
        application_receipt_records: capacity.application_records,
        canic_owned_receipt_records: capacity.canic_owned_records,
        pending_application_receipt_records: capacity.pending_records,
        terminal_application_receipt_records: capacity.terminal_records,
        receipt_record_limit: capacity.record_limit,
        remaining_receipt_record_headroom: capacity.remaining_record_headroom,
        resource_total_records: capacity.resource_total_records,
        resource_total_record_limit: capacity.resource_total_record_limit,
        remaining_resource_total_headroom: capacity.remaining_resource_total_headroom,
        warning_headroom_threshold: RECEIPT_CAPACITY_WARNING_HEADROOM_THRESHOLD,
        reserved_terminal_slots: capacity.reserved_terminal_slots,
        reserved_terminal_pages: capacity.reserved_terminal_pages,
        next_terminal_eligibility_at_ns: capacity.next_eligibility_at_ns,
        source: "intent_storage".to_string(),
    })
}

const fn receipt_capacity_condition(
    remaining_receipt_records: u64,
    remaining_resource_totals: u64,
) -> RuntimeCheckStatus {
    let minimum_headroom = if remaining_receipt_records < remaining_resource_totals {
        remaining_receipt_records
    } else {
        remaining_resource_totals
    };
    if minimum_headroom == 0 {
        RuntimeCheckStatus::Fail
    } else if minimum_headroom <= RECEIPT_CAPACITY_WARNING_HEADROOM_THRESHOLD {
        RuntimeCheckStatus::Warn
    } else {
        RuntimeCheckStatus::Pass
    }
}

const fn aggregate_runtime_status(
    readiness: ReadinessStatus,
    receipt_capacity: Option<RuntimeCheckStatus>,
    timer_inventory: RuntimeCheckStatus,
) -> RuntimeStatus {
    if matches!(readiness, ReadinessStatus::NotReady)
        || matches!(
            timer_inventory,
            RuntimeCheckStatus::Fail | RuntimeCheckStatus::NotEvaluated
        )
        || matches!(
            receipt_capacity,
            None | Some(RuntimeCheckStatus::Fail | RuntimeCheckStatus::NotEvaluated)
        )
    {
        RuntimeStatus::Failing
    } else if matches!(
        readiness,
        ReadinessStatus::Degraded | ReadinessStatus::NotEvaluated
    ) || matches!(receipt_capacity, Some(RuntimeCheckStatus::Warn))
        || matches!(timer_inventory, RuntimeCheckStatus::Warn)
    {
        RuntimeStatus::Degraded
    } else {
        RuntimeStatus::Ok
    }
}

fn runtime_blob_storage_status() -> Option<RuntimeBlobStorageStatusSummary> {
    let blob_storage_enabled = cfg!(feature = "blob-storage");
    let billing_enabled = cfg!(feature = "blob-storage-billing");

    (blob_storage_enabled || billing_enabled).then(|| RuntimeBlobStorageStatusSummary {
        blob_storage_features: [
            ("blob-storage", blob_storage_enabled),
            ("blob-storage-billing", billing_enabled),
        ]
        .into_iter()
        .map(|(name, enabled)| runtime_feature_status(name, enabled))
        .collect(),
    })
}

struct TimerStatusObservation {
    check: RuntimeCheck,
    timers: Vec<CanisterTimerStatus>,
    failure: Option<RecentFailureInput>,
}

fn timer_status_observation(
    snapshots: Result<Vec<ic_timers::TimerSnapshot>, crate::api::timer::TimerError>,
    observed_at_ns: u64,
) -> TimerStatusObservation {
    match snapshots {
        Ok(snapshots) => TimerStatusObservation {
            check: RuntimeCheck {
                category: "runtime".to_string(),
                code: "timer_inventory_available".to_string(),
                status: RuntimeCheckStatus::Pass,
                subject: "shared_timer_registry".to_string(),
                detail: "the complete canister-local timer inventory was observed".to_string(),
                next: None,
                source: "ic_timers".to_string(),
            },
            timers: timer_statuses_from(snapshots),
            failure: None,
        },
        Err(error) => TimerStatusObservation {
            check: RuntimeCheck {
                category: "runtime".to_string(),
                code: "timer_inventory_unavailable".to_string(),
                status: RuntimeCheckStatus::Fail,
                subject: "shared_timer_registry".to_string(),
                detail: "the canister-local timer inventory could not be observed".to_string(),
                next: Some(
                    "retry runtime observation; inspect recent failures if unavailable persists"
                        .to_string(),
                ),
                source: "ic_timers".to_string(),
            },
            timers: Vec::new(),
            failure: Some(RecentFailureInput {
                occurred_at_ns: observed_at_ns,
                subsystem: "timer_runtime".to_string(),
                code: "timer_inventory_unavailable".to_string(),
                severity: FailureSeverity::Error,
                summary: error.to_string(),
                correlation_id: None,
            }),
        },
    }
}

fn timer_statuses_from(snapshots: Vec<ic_timers::TimerSnapshot>) -> Vec<CanisterTimerStatus> {
    let mut timers = snapshots
        .into_iter()
        .map(|snapshot| {
            let identity = snapshot.identity();
            let observability = snapshot.observability();
            let outcomes = observability.outcomes();
            let counters = observability.counters();
            let performance = observability.performance();
            let condition = timer_process_condition(snapshot.process_condition());
            CanisterTimerStatus {
                name: identity.name().to_string(),
                owner: identity.owner().to_string(),
                subsystem: identity.subsystem().to_string(),
                scheduling_mode: timer_scheduling_mode(snapshot.scheduling_mode()),
                registration: timer_registration_status(snapshot.registration_status()),
                condition,
                enabled: condition != crate::domain::runtime::TimerProcessCondition::Disabled,
                generation: snapshot.generation(),
                next_due_at_ns: snapshot.next_deadline_ns(),
                last_outcome: outcomes.last_outcome().map(timer_execution_outcome),
                last_work_count: outcomes.last_work_count().unwrap_or_default(),
                last_success_at_ns: outcomes.last_success_at_ns(),
                last_failure_at_ns: outcomes.last_failure_at_ns(),
                consecutive_expected_failures: outcomes.consecutive_expected_failures(),
                schedules_since_runtime_start: counters.wakeups_armed(),
                executions_since_runtime_start: counters.work_started(),
                successes_since_runtime_start: counters
                    .succeeded()
                    .saturating_add(counters.no_work()),
                expected_failures_since_runtime_start: counters.retryable_failure(),
                invariant_failures_since_runtime_start: counters.invariant_failure(),
                stale_callbacks_since_runtime_start: counters
                    .stale_wakeups()
                    .saturating_add(counters.stale_work()),
                scheduler_performance: timer_callback_performance_status(
                    performance.scheduler_instructions(),
                    performance.scheduler_memory_pages(),
                ),
                work_performance: timer_callback_performance_status(
                    performance.work_instructions(),
                    performance.work_memory_pages(),
                ),
            }
        })
        .collect::<Vec<_>>();
    timers.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| left.subsystem.cmp(&right.subsystem))
            .then_with(|| left.name.cmp(&right.name))
    });
    timers
}

fn timer_callback_performance_status(
    instructions: ic_timers::MeasurementSummary,
    memory: ic_timers::MemoryPageSummary,
) -> TimerCallbackPerformanceStatus {
    TimerCallbackPerformanceStatus {
        instruction_samples_since_runtime_start: instructions.samples(),
        instructions_latest: instructions.latest(),
        instructions_maximum: instructions.maximum(),
        instructions_total_since_runtime_start: instructions.total(),
        memory_page_samples_since_runtime_start: memory.samples(),
        memory_pages_latest: memory.latest().map(timer_memory_page_sample_status),
        maximum_wasm_memory_growth_pages: memory.maximum_wasm_growth_pages(),
        maximum_stable_memory_growth_pages: memory.maximum_stable_growth_pages(),
    }
}

const fn timer_memory_page_sample_status(
    sample: ic_timers::MemoryPageSample,
) -> TimerMemoryPageSampleStatus {
    TimerMemoryPageSampleStatus {
        start: timer_memory_page_extent_status(sample.start()),
        end: timer_memory_page_extent_status(sample.end()),
    }
}

const fn timer_memory_page_extent_status(
    extent: ic_timers::MemoryPageExtent,
) -> TimerMemoryPageExtentStatus {
    TimerMemoryPageExtentStatus {
        wasm_pages: extent.wasm_pages(),
        stable_pages: extent.stable_pages(),
    }
}

const fn timer_scheduling_mode(
    value: ic_timers::TimerSchedulingMode,
) -> crate::domain::runtime::TimerSchedulingMode {
    use crate::domain::runtime::TimerSchedulingMode as Canic;
    use ic_timers::TimerSchedulingMode as Shared;
    match value {
        Shared::Once => Canic::Once,
        Shared::AfterCompletion => Canic::AfterCompletion,
        Shared::Deadline => Canic::Deadline,
        Shared::Retry => Canic::Retry,
        Shared::Continuation => Canic::Continuation,
        Shared::Watchdog => Canic::Watchdog,
    }
}

const fn timer_registration_status(
    value: ic_timers::TimerRegistrationStatus,
) -> crate::domain::runtime::TimerRegistrationStatus {
    use crate::domain::runtime::TimerRegistrationStatus as Canic;
    match value {
        ic_timers::TimerRegistrationStatus::Unregistered => Canic::Unregistered,
        ic_timers::TimerRegistrationStatus::Scheduled => Canic::Scheduled,
        ic_timers::TimerRegistrationStatus::Running => Canic::Running,
    }
}

const fn timer_process_condition(
    value: ic_timers::TimerProcessCondition,
) -> crate::domain::runtime::TimerProcessCondition {
    use crate::domain::runtime::TimerProcessCondition as Canic;
    match value {
        ic_timers::TimerProcessCondition::Disabled => Canic::Disabled,
        ic_timers::TimerProcessCondition::Idle => Canic::Idle,
        ic_timers::TimerProcessCondition::Active => Canic::Active,
        ic_timers::TimerProcessCondition::Retrying => Canic::Retrying,
        ic_timers::TimerProcessCondition::Failed => Canic::Failed,
    }
}

const fn timer_execution_outcome(
    value: ic_timers::TimerLastOutcome,
) -> crate::domain::runtime::TimerExecutionOutcome {
    use crate::domain::runtime::TimerExecutionOutcome as Canic;
    match value {
        ic_timers::TimerLastOutcome::Completed(completion) => match completion {
            ic_timers::TimerCompletionOutcome::Success => Canic::Success,
            ic_timers::TimerCompletionOutcome::NoWork => Canic::NoWork,
            ic_timers::TimerCompletionOutcome::RetryableFailure => Canic::RetryableFailure,
            ic_timers::TimerCompletionOutcome::InvariantFailure => Canic::InvariantFailure,
        },
        ic_timers::TimerLastOutcome::Unacknowledged => Canic::Unacknowledged,
    }
}

fn state_summary(role: Option<&str>) -> Option<RuntimeStateSummary> {
    let memory_ids = MemoryRegistryOps::ledger_snapshot()
        .ok()?
        .memories
        .into_iter()
        .map(|memory| memory.memory_manager_id)
        .collect::<std::collections::BTreeSet<_>>();
    state_summary_for_memory_ids(role, &memory_ids)
}

fn state_summary_for_memory_ids(
    role: Option<&str>,
    memory_ids: &std::collections::BTreeSet<u8>,
) -> Option<RuntimeStateSummary> {
    role?;
    let mut domains = canic_state_descriptors()
        .into_iter()
        .flat_map(|descriptor| descriptor.state)
        .filter(|domain| domain.memory_id.is_some_and(|id| memory_ids.contains(&id)))
        .map(|domain| RuntimeStateDomainSummary {
            domain: domain.domain,
            version: domain.version,
            storage: domain.storage.as_str().to_string(),
            memory_id: domain.memory_id,
            status: RuntimeStateDomainStatus::Ok,
        })
        .collect::<Vec<_>>();
    domains.sort_by(|left, right| left.domain.cmp(&right.domain));

    if domains.is_empty() {
        return None;
    }

    Some(RuntimeStateSummary {
        manifest_schema_version: u32::from(STATE_MANIFEST_SCHEMA_VERSION),
        domains,
        total_stable_memory_pages: None,
    })
}

fn runtime_visibility() -> Vec<RuntimeVisibilityEntry> {
    [
        ("schema_version", RuntimeFieldVisibility::PublicSafe),
        ("observed_at_ns", RuntimeFieldVisibility::PublicSafe),
        ("canister_id", RuntimeFieldVisibility::OperatorOnly),
        ("role", RuntimeFieldVisibility::OperatorOnly),
        ("root", RuntimeFieldVisibility::OperatorOnly),
        ("build_network", RuntimeFieldVisibility::OperatorOnly),
        ("build", RuntimeFieldVisibility::OperatorOnly),
        ("features", RuntimeFieldVisibility::OperatorOnly),
        ("topology", RuntimeFieldVisibility::ControllerOnly),
        ("timers", RuntimeFieldVisibility::OperatorOnly),
        ("timer_inventory", RuntimeFieldVisibility::OperatorOnly),
        ("state", RuntimeFieldVisibility::OperatorOnly),
        ("auth", RuntimeFieldVisibility::OperatorOnly),
        ("blob_storage", RuntimeFieldVisibility::FeatureGated),
        ("receipt_capacity", RuntimeFieldVisibility::OperatorOnly),
        ("recent_failures", RuntimeFieldVisibility::OperatorOnly),
        ("readiness", RuntimeFieldVisibility::OperatorOnly),
        ("status", RuntimeFieldVisibility::OperatorOnly),
        ("visibility", RuntimeFieldVisibility::OperatorOnly),
    ]
    .into_iter()
    .map(|(field, visibility)| RuntimeVisibilityEntry {
        field: field.to_string(),
        visibility,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::IntentResourceKey;
    use crate::ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps};
    use crate::ops::runtime::recent_failure::RecentFailureOps;
    use crate::ops::storage::intent::{
        INTENT_RESOURCE_TOTAL_RECORD_LIMIT, IntentStoreOps, RECEIPT_BACKED_INTENT_RECORD_LIMIT,
    };
    use crate::storage::stable::intent::{IntentResourceTotalsRecord, IntentStore};

    #[test]
    fn health_is_minimal_and_schema_versioned() {
        let health = RuntimeIntrospectionApi::health(Some(42));

        assert_eq!(health.schema_version, RUNTIME_INTROSPECTION_SCHEMA_VERSION);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.observed_at_ns, Some(42));
        assert_eq!(health.checks.len(), 1);
        assert_eq!(health.checks[0].code, "canister_responsive");
    }

    #[test]
    fn unavailable_timer_inventory_is_explicit_and_fails_runtime_status() {
        let observation =
            timer_status_observation(Err(crate::api::timer::TimerError::CustodyBusy), 101);

        assert_eq!(observation.check.status, RuntimeCheckStatus::Fail);
        assert!(observation.timers.is_empty());
        assert_eq!(
            observation
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("timer_inventory_unavailable")
        );
        assert_eq!(
            aggregate_runtime_status(
                ReadinessStatus::Ready,
                Some(RuntimeCheckStatus::Pass),
                observation.check.status,
            ),
            RuntimeStatus::Failing
        );
    }

    #[test]
    fn runtime_status_embeds_guarded_readiness_and_build_info() {
        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        assert_eq!(status.schema_version, RUNTIME_INTROSPECTION_SCHEMA_VERSION);
        assert_eq!(status.observed_at_ns, 100);
        assert_eq!(status.canister_id, Principal::anonymous());
        assert_eq!(status.build_network, BuildNetworkOps::build_network());
        assert_eq!(status.build.package_name, "test-canister");
        assert_eq!(status.build.package_version, "1.2.3");
        assert_eq!(status.build.canic_version, "0.81.0");
        assert_eq!(status.build.canister_version, 7);
        assert_eq!(status.readiness.observed_at_ns, 100);
        assert!(
            status
                .visibility
                .iter()
                .any(|entry| entry.field == "topology"
                    && entry.visibility == RuntimeFieldVisibility::ControllerOnly)
        );
    }

    #[test]
    fn runtime_status_classifies_each_top_level_field_visibility() {
        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );
        let expected = [
            ("schema_version", RuntimeFieldVisibility::PublicSafe),
            ("observed_at_ns", RuntimeFieldVisibility::PublicSafe),
            ("canister_id", RuntimeFieldVisibility::OperatorOnly),
            ("role", RuntimeFieldVisibility::OperatorOnly),
            ("root", RuntimeFieldVisibility::OperatorOnly),
            ("build_network", RuntimeFieldVisibility::OperatorOnly),
            ("build", RuntimeFieldVisibility::OperatorOnly),
            ("features", RuntimeFieldVisibility::OperatorOnly),
            ("topology", RuntimeFieldVisibility::ControllerOnly),
            ("timers", RuntimeFieldVisibility::OperatorOnly),
            ("timer_inventory", RuntimeFieldVisibility::OperatorOnly),
            ("state", RuntimeFieldVisibility::OperatorOnly),
            ("auth", RuntimeFieldVisibility::OperatorOnly),
            ("blob_storage", RuntimeFieldVisibility::FeatureGated),
            ("receipt_capacity", RuntimeFieldVisibility::OperatorOnly),
            ("recent_failures", RuntimeFieldVisibility::OperatorOnly),
            ("readiness", RuntimeFieldVisibility::OperatorOnly),
            ("status", RuntimeFieldVisibility::OperatorOnly),
            ("visibility", RuntimeFieldVisibility::OperatorOnly),
        ];

        assert_eq!(status.visibility.len(), expected.len());
        for (index, (field, visibility)) in expected.into_iter().enumerate() {
            assert_eq!(status.visibility[index].field, field);
            assert_eq!(status.visibility[index].visibility, visibility);
        }
    }

    #[test]
    fn runtime_status_projects_empty_receipt_capacity() {
        IntentStoreOps::reset_for_tests();

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.96.6",
            7,
        );
        let capacity = status.receipt_capacity.expect("receipt capacity");

        assert_eq!(capacity.status, RuntimeCheckStatus::Pass);
        assert_eq!(capacity.receipt_records, 0);
        assert_eq!(
            capacity.receipt_record_limit,
            RECEIPT_BACKED_INTENT_RECORD_LIMIT
        );
        assert_eq!(capacity.resource_total_records, 0);
        assert_eq!(
            capacity.resource_total_record_limit,
            INTENT_RESOURCE_TOTAL_RECORD_LIMIT
        );
        assert_eq!(
            capacity.warning_headroom_threshold,
            RECEIPT_CAPACITY_WARNING_HEADROOM_THRESHOLD
        );
        assert_eq!(capacity.source, "intent_storage");
    }

    #[test]
    fn receipt_capacity_condition_has_exact_warning_and_failure_boundaries() {
        assert_eq!(
            receipt_capacity_condition(RECEIPT_CAPACITY_WARNING_HEADROOM_THRESHOLD + 1, u64::MAX,),
            RuntimeCheckStatus::Pass
        );
        assert_eq!(
            receipt_capacity_condition(RECEIPT_CAPACITY_WARNING_HEADROOM_THRESHOLD, u64::MAX),
            RuntimeCheckStatus::Warn
        );
        assert_eq!(
            receipt_capacity_condition(u64::MAX, 1),
            RuntimeCheckStatus::Warn
        );
        assert_eq!(
            receipt_capacity_condition(u64::MAX, 0),
            RuntimeCheckStatus::Fail
        );
        assert_eq!(
            aggregate_runtime_status(
                ReadinessStatus::Ready,
                Some(RuntimeCheckStatus::Warn),
                RuntimeCheckStatus::Pass,
            ),
            RuntimeStatus::Degraded
        );
        assert_eq!(
            aggregate_runtime_status(ReadinessStatus::Ready, None, RuntimeCheckStatus::Pass,),
            RuntimeStatus::Failing
        );
        assert_eq!(
            aggregate_runtime_status(
                ReadinessStatus::Ready,
                Some(RuntimeCheckStatus::Pass),
                RuntimeCheckStatus::Fail,
            ),
            RuntimeStatus::Failing
        );
    }

    #[test]
    fn runtime_status_fails_closed_with_typed_capacity_diagnostic() {
        IntentStoreOps::reset_for_tests();
        RecentFailureOps::reset();
        for value in 0..=INTENT_RESOURCE_TOTAL_RECORD_LIMIT {
            IntentStore::set_totals(
                IntentResourceKey::new(format!("runtime-capacity:{value}")),
                IntentResourceTotalsRecord {
                    reserved_qty: 0,
                    committed_qty: 1,
                    pending_count: 0,
                },
            );
        }

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.96.6",
            7,
        );

        assert_eq!(status.status, RuntimeStatus::Failing);
        assert!(status.receipt_capacity.is_none());
        let expected_code = crate::diagnostics::codes::CAPACITY_LIMIT.to_string();
        let failure = status
            .recent_failures
            .iter()
            .find(|failure| failure.code == expected_code)
            .expect("current capacity failure diagnostic");
        assert_eq!(failure.subsystem, "intent_capacity");
        assert_eq!(failure.code, expected_code);
        assert_eq!(failure.severity, FailureSeverity::Error);
        assert_eq!(
            failure.summary,
            format!("diagnostic={}", crate::diagnostics::codes::CAPACITY_LIMIT)
        );
        assert!(RecentFailureOps::snapshot().is_empty());

        IntentStoreOps::reset_for_tests();
        RecentFailureOps::reset();
    }

    #[test]
    fn runtime_status_reports_compile_features_deterministically() {
        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );
        assert_eq!(status.features.len(), RUNTIME_FEATURE_FLAGS.len());
        for (index, (name, enabled)) in RUNTIME_FEATURE_FLAGS.into_iter().enumerate() {
            assert_eq!(status.features[index].name, name);
            assert_eq!(status.features[index].enabled, enabled);
            assert_eq!(
                status.features[index].visibility,
                RuntimeFieldVisibility::OperatorOnly
            );
            assert_eq!(status.features[index].source, RUNTIME_FEATURE_SOURCE);
        }
    }

    #[test]
    fn runtime_status_reports_auth_and_blob_storage_feature_summaries() {
        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        let auth = status.auth.expect("auth feature summary");
        assert!(
            auth.auth_features
                .windows(2)
                .all(|features| features[0].name <= features[1].name)
        );
        assert_runtime_feature(
            &auth.auth_features,
            "auth-chain-key-ecdsa",
            cfg!(feature = "auth-chain-key-ecdsa"),
        );
        assert_runtime_feature(
            &auth.auth_features,
            "auth-delegated-token-verify",
            cfg!(feature = "auth-delegated-token-verify"),
        );
        assert_runtime_feature(
            &auth.auth_features,
            "auth-issuer-canister-sig-create",
            cfg!(feature = "auth-issuer-canister-sig-create"),
        );

        if cfg!(any(
            feature = "blob-storage",
            feature = "blob-storage-billing"
        )) {
            let blob_storage = status.blob_storage.expect("blob-storage feature summary");
            assert_runtime_feature(
                &blob_storage.blob_storage_features,
                "blob-storage",
                cfg!(feature = "blob-storage"),
            );
            assert_runtime_feature(
                &blob_storage.blob_storage_features,
                "blob-storage-billing",
                cfg!(feature = "blob-storage-billing"),
            );
        } else {
            assert!(status.blob_storage.is_none());
        }
    }

    fn assert_runtime_feature(
        features: &[RuntimeFeatureStatus],
        name: &str,
        expected_enabled: bool,
    ) {
        let feature = features
            .iter()
            .find(|feature| feature.name == name)
            .unwrap_or_else(|| panic!("expected runtime feature {name}"));

        assert_eq!(feature.enabled, expected_enabled);
        assert_eq!(feature.visibility, RuntimeFieldVisibility::OperatorOnly);
        assert_eq!(feature.source, RUNTIME_FEATURE_SOURCE);
    }

    #[test]
    fn state_summary_joins_runtime_memory_ids_to_owner_metadata() {
        let summary = state_summary_for_memory_ids(
            Some("root"),
            &std::collections::BTreeSet::from([
                crate::role_contract::allocation::memory::runtime::RUNTIME_BINDINGS_ID,
            ]),
        )
        .expect("runtime state declarations");

        assert_eq!(
            summary.manifest_schema_version,
            u32::from(crate::state_contract::STATE_MANIFEST_SCHEMA_VERSION)
        );
        assert!(summary.total_stable_memory_pages.is_none());
        assert!(summary.domains.iter().any(|domain| {
            domain.domain == "runtime_bindings"
                && domain.storage == "stable_memory"
                && domain.status == RuntimeStateDomainStatus::Ok
        }));
        assert!(state_summary_for_memory_ids(None, &std::collections::BTreeSet::new()).is_none());
    }

    #[test]
    fn runtime_status_includes_recent_failure_snapshot() {
        RecentFailureOps::reset();
        RecentFailureOps::record(RecentFailureInput {
            occurred_at_ns: 77,
            subsystem: "runtime".to_string(),
            code: "readiness_failed".to_string(),
            severity: FailureSeverity::Error,
            summary: "bounded failure summary".to_string(),
            correlation_id: Some("runtime-check".to_string()),
        });

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        let failure = status
            .recent_failures
            .iter()
            .find(|failure| failure.code == "readiness_failed")
            .expect("retained recent failure");
        assert_eq!(failure.occurred_at_ns, 77);
        assert_eq!(failure.subsystem, "runtime");

        RecentFailureOps::reset();
    }

    #[test]
    fn runtime_status_includes_bootstrap_failure_metadata() {
        RecentFailureOps::reset();
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT);
        BootstrapStatusOps::mark_failed("raw bootstrap failure detail");

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        let failure = status
            .recent_failures
            .iter()
            .find(|failure| failure.code == "bootstrap_failed")
            .expect("bootstrap failure metadata");

        assert_eq!(failure.subsystem, "runtime_bootstrap");
        assert_eq!(failure.severity, FailureSeverity::Error);
        assert_eq!(failure.correlation_id.as_deref(), Some("root:init"));
        assert!(
            !failure.summary.contains("raw bootstrap failure detail"),
            "runtime status recent failures should not mirror raw bootstrap errors"
        );

        RecentFailureOps::reset();
    }
}
