pub mod install;

use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        runtime::{
            CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, CanicTimerStatus,
            FailureSeverity, RUNTIME_INTROSPECTION_SCHEMA_VERSION, ReadinessStatus,
            RuntimeBuildInfo, RuntimeCheck, RuntimeCheckStatus, RuntimeDiagnostic,
            RuntimeDiagnosticSeverity, RuntimeFeatureStatus, RuntimeFieldVisibility,
            RuntimeStateDomainStatus, RuntimeStateDomainSummary, RuntimeStateSummary,
            RuntimeStatus, RuntimeTopologyStatus, RuntimeVisibilityEntry, TimerStatus,
        },
    },
    ops::{
        ic::IcOps,
        runtime::{
            env::EnvOps,
            memory::MemoryRegistryOps,
            metrics::timer::TimerMetrics,
            ready::ReadyOps,
            recent_failure::{RecentFailureInput, RecentFailureOps},
        },
    },
    state_contract::{StateStorage, canic_state_manifest_for_role},
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
            status: crate::dto::runtime::HealthStatus::Healthy,
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
            network: None,
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
        .map(|(name, enabled)| RuntimeFeatureStatus {
            name: name.to_string(),
            enabled,
            visibility: RuntimeFieldVisibility::OperatorOnly,
            source: RUNTIME_FEATURE_SOURCE.to_string(),
        })
        .collect()
}

fn timer_statuses() -> Vec<CanicTimerStatus> {
    let mut timers = TimerMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, ticks)| {
            let (subsystem, name) = split_timer_label(&key.label);
            CanicTimerStatus {
                name,
                subsystem,
                status: if ticks > 0 {
                    TimerStatus::Healthy
                } else {
                    TimerStatus::Unknown
                },
                enabled: true,
                registered: true,
                last_success_at_ns: None,
                last_failure_at_ns: None,
                next_due_at_ns: None,
                consecutive_failures: 0,
                last_error_code: None,
                last_error_summary: None,
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
    let role = role?;
    let manifest = canic_state_manifest_for_role(Some(role));
    let domains = manifest
        .roles
        .into_iter()
        .flat_map(|role| role.state)
        .map(|domain| RuntimeStateDomainSummary {
            domain: domain.domain,
            version: domain.version,
            storage: state_storage_name(domain.storage).to_string(),
            memory_id: domain.memory_id,
            status: RuntimeStateDomainStatus::Ok,
        })
        .collect::<Vec<_>>();

    if domains.is_empty() {
        return None;
    }

    Some(RuntimeStateSummary {
        manifest_schema_version: u32::from(manifest.schema_version),
        domains,
        total_stable_memory_pages: None,
    })
}

const fn state_storage_name(storage: StateStorage) -> &'static str {
    match storage {
        StateStorage::StableMemory => "stable_memory",
        StateStorage::HeapOnly => "heap_only",
        StateStorage::NotApplicable => "not_applicable",
    }
}

fn split_timer_label(label: &str) -> (String, String) {
    label.split_once(':').map_or_else(
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
        ("network", RuntimeFieldVisibility::OperatorOnly),
        ("build", RuntimeFieldVisibility::OperatorOnly),
        ("features", RuntimeFieldVisibility::OperatorOnly),
        ("topology", RuntimeFieldVisibility::ControllerOnly),
        ("timers", RuntimeFieldVisibility::OperatorOnly),
        ("state", RuntimeFieldVisibility::OperatorOnly),
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
    use crate::ops::runtime::metrics::timer::{TimerMetrics, TimerMode};
    use crate::ops::runtime::recent_failure::RecentFailureOps;
    use std::time::Duration;

    #[test]
    fn health_is_minimal_and_schema_versioned() {
        let health = RuntimeIntrospectionApi::health(Some(42));

        assert_eq!(health.schema_version, RUNTIME_INTROSPECTION_SCHEMA_VERSION);
        assert_eq!(health.status, crate::dto::runtime::HealthStatus::Healthy);
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
            ("network", RuntimeFieldVisibility::OperatorOnly),
            ("build", RuntimeFieldVisibility::OperatorOnly),
            ("features", RuntimeFieldVisibility::OperatorOnly),
            ("topology", RuntimeFieldVisibility::ControllerOnly),
            ("timers", RuntimeFieldVisibility::OperatorOnly),
            ("state", RuntimeFieldVisibility::OperatorOnly),
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
    fn runtime_status_projects_registered_timer_metrics() {
        TimerMetrics::reset();
        TimerMetrics::record_timer_scheduled(
            TimerMode::Interval,
            Duration::from_mins(1),
            "cycles:interval",
        );
        TimerMetrics::record_timer_scheduled(
            TimerMode::Once,
            Duration::from_secs(1),
            "auth_renewal:init",
        );
        TimerMetrics::record_timer_tick(
            TimerMode::Once,
            Duration::from_secs(1),
            "auth_renewal:init",
        );

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        assert_eq!(status.timers.len(), 2);
        assert_eq!(status.timers[0].subsystem, "auth_renewal");
        assert_eq!(status.timers[0].name, "init");
        assert_eq!(status.timers[0].status, TimerStatus::Healthy);
        assert_eq!(status.timers[1].subsystem, "cycles");
        assert_eq!(status.timers[1].name, "interval");
        assert_eq!(status.timers[1].status, TimerStatus::Unknown);

        TimerMetrics::reset();
    }

    #[test]
    fn runtime_status_bounds_timer_labels() {
        TimerMetrics::reset();

        let label = format!("{}\n:{}\n", "subsystem".repeat(12), "timer_name".repeat(16));
        TimerMetrics::record_timer_scheduled(TimerMode::Once, Duration::from_secs(1), &label);

        let status = RuntimeIntrospectionApi::runtime_status_for(
            Principal::anonymous(),
            100,
            "test-canister",
            "1.2.3",
            "0.81.0",
            7,
        );

        assert_eq!(status.timers.len(), 1);
        assert!(status.timers[0].subsystem.len() <= MAX_TIMER_SUBSYSTEM_BYTES);
        assert!(status.timers[0].name.len() <= MAX_TIMER_NAME_BYTES);
        assert!(!status.timers[0].subsystem.contains('\n'));
        assert!(!status.timers[0].name.contains('\n'));

        TimerMetrics::reset();
    }

    #[test]
    fn state_summary_uses_declared_metadata_without_value_counts() {
        let summary = state_summary(Some("root")).expect("root state declarations");

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
        assert!(state_summary(Some("unknown_role")).is_none());
        assert!(state_summary(None).is_none());
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
}
