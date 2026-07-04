pub mod install;

use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        runtime::{
            CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, CanicTimerStatus,
            FailureSeverity, RUNTIME_INTROSPECTION_SCHEMA_VERSION, ReadinessStatus,
            RuntimeBuildInfo, RuntimeCheck, RuntimeCheckStatus, RuntimeDiagnostic,
            RuntimeDiagnosticSeverity, RuntimeFieldVisibility, RuntimeStateDomainStatus,
            RuntimeStateDomainSummary, RuntimeStateSummary, RuntimeStatus, RuntimeTopologyStatus,
            RuntimeVisibilityEntry, TimerStatus,
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
            features: Vec::new(),
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
        || ("runtime".to_string(), label.to_string()),
        |(subsystem, name)| (subsystem.to_string(), name.to_string()),
    )
}

fn runtime_visibility() -> Vec<RuntimeVisibilityEntry> {
    vec![
        RuntimeVisibilityEntry {
            field: "schema_version".to_string(),
            visibility: RuntimeFieldVisibility::PublicSafe,
        },
        RuntimeVisibilityEntry {
            field: "status".to_string(),
            visibility: RuntimeFieldVisibility::OperatorOnly,
        },
        RuntimeVisibilityEntry {
            field: "role".to_string(),
            visibility: RuntimeFieldVisibility::OperatorOnly,
        },
        RuntimeVisibilityEntry {
            field: "root".to_string(),
            visibility: RuntimeFieldVisibility::OperatorOnly,
        },
        RuntimeVisibilityEntry {
            field: "topology".to_string(),
            visibility: RuntimeFieldVisibility::ControllerOnly,
        },
        RuntimeVisibilityEntry {
            field: "timers".to_string(),
            visibility: RuntimeFieldVisibility::OperatorOnly,
        },
        RuntimeVisibilityEntry {
            field: "state".to_string(),
            visibility: RuntimeFieldVisibility::OperatorOnly,
        },
        RuntimeVisibilityEntry {
            field: "recent_failures".to_string(),
            visibility: RuntimeFieldVisibility::OperatorOnly,
        },
    ]
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
    fn runtime_status_projects_registered_timer_metrics() {
        TimerMetrics::reset();
        TimerMetrics::record_timer_scheduled(
            TimerMode::Interval,
            Duration::from_secs(60),
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
