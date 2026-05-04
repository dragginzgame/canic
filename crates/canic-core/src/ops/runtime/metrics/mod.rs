pub mod access;
pub mod auth;
pub mod cycles_funding;
pub mod cycles_topup;
pub mod delegated_auth;
pub mod http;
pub mod icc;
pub mod lifecycle;
pub mod root_capability;
pub mod system;
pub mod timer;

use crate::{
    dto::metrics::{MetricEntry, MetricValue, MetricsKind},
    perf::{self, PerfKey},
};
use {
    access::AccessMetrics,
    cycles_funding::CyclesFundingMetrics,
    cycles_topup::CyclesTopupMetrics,
    delegated_auth::DelegatedAuthMetrics,
    http::HttpMetrics,
    icc::IccMetrics,
    lifecycle::LifecycleMetrics,
    root_capability::RootCapabilityMetrics,
    system::SystemMetrics,
    timer::{TimerMetrics, TimerMode},
};

/// Project one metrics family into the unified public metrics row shape.
#[must_use]
pub fn entries(kind: MetricsKind) -> Vec<MetricEntry> {
    match kind {
        MetricsKind::Access => access_entries(),
        MetricsKind::CyclesFunding => cycles_funding_entries(),
        MetricsKind::CyclesTopup => cycles_topup_entries(),
        MetricsKind::DelegatedAuth => delegated_auth_entries(),
        MetricsKind::Http => http_entries(),
        MetricsKind::Icc => icc_entries(),
        MetricsKind::Lifecycle => lifecycle_entries(),
        MetricsKind::Perf => perf_entries(),
        MetricsKind::RootCapability => root_capability_entries(),
        MetricsKind::System => system_entries(),
        MetricsKind::Timer => timer_entries(),
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    AccessMetrics::reset();
    CyclesFundingMetrics::reset();
    CyclesTopupMetrics::reset();
    DelegatedAuthMetrics::reset();
    HttpMetrics::reset();
    IccMetrics::reset();
    LifecycleMetrics::reset();
    RootCapabilityMetrics::reset();
    SystemMetrics::reset();
    TimerMetrics::reset();
    perf::reset();
}

/// Project system counters into the unified public metrics row shape.
#[must_use]
fn system_entries() -> Vec<MetricEntry> {
    SystemMetrics::snapshot()
        .into_iter()
        .map(|(kind, count)| MetricEntry {
            labels: vec![
                match kind {
                    crate::ids::SystemMetricKind::CanisterCall => "CanisterCall",
                    crate::ids::SystemMetricKind::CanisterStatus => "CanisterStatus",
                    crate::ids::SystemMetricKind::CreateCanister => "CreateCanister",
                    crate::ids::SystemMetricKind::DeleteCanister => "DeleteCanister",
                    crate::ids::SystemMetricKind::DepositCycles => "DepositCycles",
                    crate::ids::SystemMetricKind::HttpOutcall => "HttpOutcall",
                    crate::ids::SystemMetricKind::InstallCode => "InstallCode",
                    crate::ids::SystemMetricKind::RawRand => "RawRand",
                    crate::ids::SystemMetricKind::ReinstallCode => "ReinstallCode",
                    crate::ids::SystemMetricKind::TimerScheduled => "TimerScheduled",
                    crate::ids::SystemMetricKind::UninstallCode => "UninstallCode",
                    crate::ids::SystemMetricKind::UpdateSettings => "UpdateSettings",
                    crate::ids::SystemMetricKind::UpgradeCode => "UpgradeCode",
                }
                .to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project lifecycle counters into the unified public metrics row shape.
#[must_use]
fn lifecycle_entries() -> Vec<MetricEntry> {
    LifecycleMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.phase.metric_label().to_string(),
                key.role.metric_label().to_string(),
                key.stage.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project inter-canister call counters into the unified public metrics row shape.
#[must_use]
fn icc_entries() -> Vec<MetricEntry> {
    IccMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![key.method],
            principal: Some(key.target),
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project HTTP outcall counters into the unified public metrics row shape.
#[must_use]
fn http_entries() -> Vec<MetricEntry> {
    HttpMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![key.method.as_str().to_string(), key.label],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project timer counters into the unified public metrics row shape.
#[must_use]
fn timer_entries() -> Vec<MetricEntry> {
    TimerMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                match key.mode {
                    TimerMode::Once => "once",
                    TimerMode::Interval => "interval",
                }
                .to_string(),
                key.label,
            ],
            principal: None,
            value: MetricValue::CountAndU64 {
                count,
                value_u64: key.delay_ms,
            },
        })
        .collect()
}

/// Project access-denial counters into the unified public metrics row shape.
#[must_use]
fn access_entries() -> Vec<MetricEntry> {
    AccessMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![key.endpoint, key.kind.as_str().to_string(), key.predicate],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project delegated-auth authority counters into the unified public metrics row shape.
#[must_use]
fn delegated_auth_entries() -> Vec<MetricEntry> {
    DelegatedAuthMetrics::snapshot()
        .into_iter()
        .map(|(authority, count)| MetricEntry {
            labels: vec!["delegated_auth_authority".to_string()],
            principal: Some(authority),
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project root-capability counters into the unified public metrics row shape.
#[must_use]
fn root_capability_entries() -> Vec<MetricEntry> {
    RootCapabilityMetrics::snapshot()
        .into_iter()
        .map(
            |(capability, event_type, outcome, proof_mode, count)| MetricEntry {
                labels: vec![
                    capability.metric_label().to_string(),
                    event_type.metric_label().to_string(),
                    outcome.metric_label().to_string(),
                    proof_mode.metric_label().to_string(),
                ],
                principal: None,
                value: MetricValue::Count(count),
            },
        )
        .collect()
}

/// Project cycles-funding counters into the unified public metrics row shape.
#[must_use]
fn cycles_funding_entries() -> Vec<MetricEntry> {
    CyclesFundingMetrics::snapshot()
        .into_iter()
        .map(|(metric, child_principal, reason, cycles)| MetricEntry {
            labels: reason.map_or_else(
                || vec![metric.metric_label().to_string()],
                |reason| {
                    vec![
                        metric.metric_label().to_string(),
                        reason.metric_label().to_string(),
                    ]
                },
            ),
            principal: child_principal,
            value: MetricValue::U128(cycles),
        })
        .collect()
}

/// Project auto-top-up decision counters into the unified public metrics row shape.
#[must_use]
fn cycles_topup_entries() -> Vec<MetricEntry> {
    CyclesTopupMetrics::snapshot()
        .into_iter()
        .map(|(metric, count)| MetricEntry {
            labels: vec![metric.metric_label().to_string()],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project perf counters into the unified public metrics row shape.
#[must_use]
fn perf_entries() -> Vec<MetricEntry> {
    perf::entries()
        .into_iter()
        .map(|entry| {
            let labels = match entry.key {
                PerfKey::Endpoint(label) => vec!["endpoint".to_string(), label],
                PerfKey::Timer(label) => vec!["timer".to_string(), label],
                PerfKey::Checkpoint { scope, label } => {
                    vec!["checkpoint".to_string(), scope, label]
                }
            };

            MetricEntry {
                labels,
                principal: None,
                value: MetricValue::CountAndU64 {
                    count: entry.count,
                    value_u64: entry.total_instructions,
                },
            }
        })
        .collect()
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        ids::AccessMetricKind,
        ops::runtime::metrics::{
            cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
            cycles_topup::CyclesTopupMetrics,
            http::{HttpMethod, HttpMetrics},
            icc::IccMetrics,
            lifecycle::{
                LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole,
                LifecycleMetricStage, LifecycleMetrics,
            },
            root_capability::{
                RootCapabilityMetricKey, RootCapabilityMetricOutcome,
                RootCapabilityMetricProofMode, RootCapabilityMetrics,
            },
            timer::{TimerMetrics, TimerMode},
        },
    };
    use std::time::Duration;

    #[test]
    fn cycles_topup_metrics_are_exposed() {
        reset_for_tests();

        CyclesTopupMetrics::record_policy_missing();
        CyclesTopupMetrics::record_request_scheduled();
        CyclesTopupMetrics::record_request_scheduled();

        let entries = entries(MetricsKind::CyclesTopup);

        assert_metric_count(&entries, &["policy_missing"], 1);
        assert_metric_count(&entries, &["request_scheduled"], 2);
    }

    #[test]
    fn reset_for_tests_clears_all_metric_families() {
        reset_for_tests();
        let principal = Principal::from_slice(&[42; 29]);

        AccessMetrics::increment("create_project", AccessMetricKind::Guard, "controller_only");
        CyclesFundingMetrics::record_denied(
            principal,
            10,
            CyclesFundingDeniedReason::ChildNotFound,
        );
        CyclesTopupMetrics::record_request_scheduled();
        DelegatedAuthMetrics::record_authority(principal);
        HttpMetrics::record_http_request(HttpMethod::Get, "https://example.test/a", Some("api"));
        IccMetrics::record_call(principal, "canic_sync");
        LifecycleMetrics::record(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricStage::Bootstrap,
            LifecycleMetricOutcome::Started,
        );
        RootCapabilityMetrics::record_proof(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
            RootCapabilityMetricProofMode::Structural,
        );
        TimerMetrics::record_timer_scheduled(TimerMode::Once, Duration::from_secs(1), "once:test");
        perf::record_checkpoint("metrics::tests", "checkpoint", 7);

        for kind in [
            MetricsKind::Access,
            MetricsKind::CyclesFunding,
            MetricsKind::CyclesTopup,
            MetricsKind::DelegatedAuth,
            MetricsKind::Http,
            MetricsKind::Icc,
            MetricsKind::Lifecycle,
            MetricsKind::Perf,
            MetricsKind::RootCapability,
            MetricsKind::System,
            MetricsKind::Timer,
        ] {
            assert!(!entries(kind).is_empty());
        }

        reset_for_tests();

        for kind in [
            MetricsKind::Access,
            MetricsKind::CyclesFunding,
            MetricsKind::CyclesTopup,
            MetricsKind::DelegatedAuth,
            MetricsKind::Http,
            MetricsKind::Icc,
            MetricsKind::Lifecycle,
            MetricsKind::Perf,
            MetricsKind::RootCapability,
            MetricsKind::System,
            MetricsKind::Timer,
        ] {
            assert!(entries(kind).is_empty());
        }
    }

    fn assert_metric_count(entries: &[MetricEntry], labels: &[&str], expected: u64) {
        let entry = entries
            .iter()
            .find(|entry| entry.labels.iter().map(String::as_str).collect::<Vec<_>>() == labels)
            .expect("metric entry should exist");

        match &entry.value {
            MetricValue::Count(count) => assert_eq!(*count, expected),
            _ => panic!("metric entry should use Count"),
        }
    }
}
