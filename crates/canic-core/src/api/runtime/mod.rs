pub mod install;

use crate::{
    cdk::types::Principal,
    domain::runtime::{
        FailureSeverity, HealthStatus, ReadinessStatus, RuntimeCheckStatus,
        RuntimeDiagnosticSeverity, RuntimeFieldVisibility, RuntimeStateDomainStatus, RuntimeStatus,
    },
    dto::{
        error::Error,
        runtime::{
            CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, CanicTimerStatus,
            RUNTIME_INTROSPECTION_SCHEMA_VERSION, RuntimeAuthStatusSummary,
            RuntimeBlobStorageStatusSummary, RuntimeBuildInfo, RuntimeCheck, RuntimeDiagnostic,
            RuntimeFeatureStatus, RuntimeStateDomainSummary, RuntimeStateSummary,
            RuntimeTopologyStatus, RuntimeVisibilityEntry,
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
    },
    state_contract::{STATE_MANIFEST_SCHEMA_VERSION, canic_state_descriptors},
    workflow::runtime::timer::{TimerRuntimeSnapshot, TimerWorkflow},
};

const MAX_TIMER_SUBSYSTEM_BYTES: usize = 64;
const MAX_TIMER_NAME_BYTES: usize = 96;
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
    /// Record one heap-only recent-failure summary for guarded runtime status.
    pub fn record_recent_failure(
        occurred_at_ns: u64,
        subsystem: impl Into<String>,
        code: impl Into<String>,
        severity: FailureSeverity,
        summary: impl Into<String>,
        correlation_id: Option<String>,
    ) {
        RecentFailureOps::record(RecentFailureInput {
            occurred_at_ns,
            subsystem: subsystem.into(),
            code: code.into(),
            severity,
            summary: summary.into(),
            correlation_id,
        });
    }

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
        let status = match readiness.status {
            ReadinessStatus::Ready => RuntimeStatus::Ok,
            ReadinessStatus::Degraded | ReadinessStatus::NotEvaluated => RuntimeStatus::Degraded,
            ReadinessStatus::NotReady => RuntimeStatus::Failing,
        };

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
            timers: timer_statuses(),
            state,
            auth: Some(runtime_auth_status()),
            blob_storage: runtime_blob_storage_status(),
            recent_failures: RecentFailureOps::snapshot(),
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

fn timer_statuses() -> Vec<CanicTimerStatus> {
    timer_statuses_from(TimerWorkflow::statuses())
}

fn timer_statuses_from(snapshots: Vec<TimerRuntimeSnapshot>) -> Vec<CanicTimerStatus> {
    let mut timers = snapshots
        .into_iter()
        .map(|snapshot| {
            let (subsystem, name) = split_timer_label(&snapshot.label);
            CanicTimerStatus {
                name,
                subsystem,
                scheduling_mode: snapshot.scheduling_mode,
                registration: snapshot.registration,
                condition: snapshot.condition,
                enabled: snapshot.enabled,
                generation: snapshot.generation,
                next_due_at_ns: snapshot.next_due_at_ns,
                last_outcome: snapshot.last_outcome,
                last_work_count: snapshot.last_work_count,
                last_success_at_ns: snapshot.last_success_at_ns,
                last_failure_at_ns: snapshot.last_failure_at_ns,
                consecutive_expected_failures: snapshot.consecutive_expected_failures,
                schedules_since_runtime_start: snapshot.schedules_since_runtime_start,
                executions_since_runtime_start: snapshot.executions_since_runtime_start,
                successes_since_runtime_start: snapshot.successes_since_runtime_start,
                expected_failures_since_runtime_start: snapshot
                    .expected_failures_since_runtime_start,
                invariant_failures_since_runtime_start: snapshot
                    .invariant_failures_since_runtime_start,
                stale_callbacks_since_runtime_start: snapshot.stale_callbacks_since_runtime_start,
            }
        })
        .collect::<Vec<_>>();
    timers.sort_by(|left, right| {
        left.subsystem
            .cmp(&right.subsystem)
            .then_with(|| left.name.cmp(&right.name))
    });
    timers
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

fn split_timer_label(label: &str) -> (String, String) {
    label
        .split_once("::")
        .or_else(|| label.split_once(':'))
        .map_or_else(
            || {
                (
                    "runtime".to_string(),
                    bounded_runtime_text(label, MAX_TIMER_NAME_BYTES),
                )
            },
            |(subsystem, name)| {
                (
                    bounded_runtime_text(subsystem, MAX_TIMER_SUBSYSTEM_BYTES),
                    bounded_runtime_text(name, MAX_TIMER_NAME_BYTES),
                )
            },
        )
}

fn bounded_runtime_text(value: &str, max_bytes: usize) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();

    if sanitized.len() <= max_bytes {
        return sanitized;
    }

    let mut end = 0;
    for (index, character) in sanitized.char_indices() {
        let next = index + character.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }

    sanitized[..end].to_string()
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
        ("state", RuntimeFieldVisibility::OperatorOnly),
        ("auth", RuntimeFieldVisibility::OperatorOnly),
        ("blob_storage", RuntimeFieldVisibility::FeatureGated),
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
    use crate::domain::runtime::{
        TimerExecutionOutcome, TimerProcessCondition, TimerRegistrationStatus, TimerSchedulingMode,
    };
    use crate::ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps};
    use crate::ops::runtime::recent_failure::RecentFailureOps;

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
            ("state", RuntimeFieldVisibility::OperatorOnly),
            ("auth", RuntimeFieldVisibility::OperatorOnly),
            ("blob_storage", RuntimeFieldVisibility::FeatureGated),
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
    fn runtime_status_projects_live_registration_and_process_condition() {
        let statuses = timer_statuses_from(vec![timer_snapshot("cycles:tracking")]);

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].subsystem, "cycles");
        assert_eq!(statuses[0].name, "tracking");
        assert_eq!(statuses[0].registration, TimerRegistrationStatus::Scheduled);
        assert_eq!(statuses[0].condition, TimerProcessCondition::Active);
        assert_eq!(
            statuses[0].last_outcome,
            Some(TimerExecutionOutcome::Success)
        );
        assert_eq!(statuses[0].schedules_since_runtime_start, 3);
    }

    #[test]
    fn runtime_status_bounds_timer_labels() {
        let label = format!("{}\n:{}\n", "subsystem".repeat(12), "timer_name".repeat(16));
        let status = timer_statuses_from(vec![timer_snapshot(&label)]);

        assert_eq!(status.len(), 1);
        assert!(status[0].subsystem.len() <= MAX_TIMER_SUBSYSTEM_BYTES);
        assert!(status[0].name.len() <= MAX_TIMER_NAME_BYTES);
        assert!(!status[0].subsystem.contains('\n'));
        assert!(!status[0].name.contains('\n'));
    }

    fn timer_snapshot(label: &str) -> TimerRuntimeSnapshot {
        TimerRuntimeSnapshot {
            label: label.to_string(),
            scheduling_mode: TimerSchedulingMode::AfterCompletion,
            registration: TimerRegistrationStatus::Scheduled,
            condition: TimerProcessCondition::Active,
            enabled: true,
            generation: 4,
            next_due_at_ns: Some(500),
            last_outcome: Some(TimerExecutionOutcome::Success),
            last_work_count: 2,
            last_success_at_ns: Some(400),
            last_failure_at_ns: None,
            consecutive_expected_failures: 0,
            schedules_since_runtime_start: 3,
            executions_since_runtime_start: 2,
            successes_since_runtime_start: 2,
            expected_failures_since_runtime_start: 0,
            invariant_failures_since_runtime_start: 0,
            stale_callbacks_since_runtime_start: 0,
        }
    }

    #[test]
    fn state_summary_joins_runtime_memory_ids_to_owner_metadata() {
        let summary = state_summary_for_memory_ids(
            Some("root"),
            &std::collections::BTreeSet::from([
                crate::role_contract::allocation::memory::env::ENV_ID,
            ]),
        )
        .expect("runtime state declarations");

        assert_eq!(
            summary.manifest_schema_version,
            u32::from(crate::state_contract::STATE_MANIFEST_SCHEMA_VERSION)
        );
        assert!(summary.total_stable_memory_pages.is_none());
        assert!(summary.domains.iter().any(|domain| {
            domain.domain == "env"
                && domain.storage == "stable_memory"
                && domain.status == RuntimeStateDomainStatus::Ok
        }));
        assert!(state_summary_for_memory_ids(None, &std::collections::BTreeSet::new()).is_none());
    }

    #[test]
    fn runtime_status_includes_recent_failure_snapshot() {
        RecentFailureOps::reset();
        RuntimeIntrospectionApi::record_recent_failure(
            77,
            "runtime",
            "readiness_failed",
            FailureSeverity::Error,
            "bounded failure summary",
            Some("runtime-check".to_string()),
        );

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        assert_eq!(status.recent_failures.len(), 1);
        assert_eq!(status.recent_failures[0].occurred_at_ns, 77);
        assert_eq!(status.recent_failures[0].subsystem, "runtime");
        assert_eq!(status.recent_failures[0].code, "readiness_failed");

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
