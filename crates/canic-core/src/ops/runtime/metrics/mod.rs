pub mod access;
pub mod auth;
pub mod cycles_funding;
pub mod delegation;
pub mod endpoint;
pub mod http;
pub mod icc;
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
    delegation::DelegationMetrics,
    http::HttpMetrics,
    icc::IccMetrics,
    root_capability::RootCapabilityMetrics,
    system::SystemMetrics,
    timer::{TimerMetrics, TimerMode},
};

/// Project one metrics family into the unified public metrics row shape.
#[must_use]
pub fn entries(kind: MetricsKind) -> Vec<MetricEntry> {
    match kind {
        MetricsKind::System => system_entries(),
        MetricsKind::Icc => icc_entries(),
        MetricsKind::Http => http_entries(),
        MetricsKind::Timer => timer_entries(),
        MetricsKind::Access => access_entries(),
        MetricsKind::Delegation => delegation_entries(),
        MetricsKind::RootCapability => root_capability_entries(),
        MetricsKind::CyclesFunding => cycles_funding_entries(),
        MetricsKind::Perf => perf_entries(),
    }
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

/// Project delegation authority counters into the unified public metrics row shape.
#[must_use]
fn delegation_entries() -> Vec<MetricEntry> {
    DelegationMetrics::snapshot()
        .into_iter()
        .map(|(authority, count)| MetricEntry {
            labels: vec!["delegation_authority".to_string()],
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

/// Project perf counters into the unified public metrics row shape.
#[must_use]
fn perf_entries() -> Vec<MetricEntry> {
    perf::entries()
        .into_iter()
        .map(|entry| {
            let labels = match entry.key {
                PerfKey::Endpoint(label) => vec!["endpoint".to_string(), label],
                PerfKey::Timer(label) => vec!["timer".to_string(), label],
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
