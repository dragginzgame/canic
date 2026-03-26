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
    dto::metrics::{MetricEntry, MetricsKind},
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

///
/// MetricsOps
///
/// Aggregation facade over the ops-internal metrics subsystems.
///
/// NOTE:
/// Individual metrics (AccessMetrics, HttpMetrics, IccMetrics, etc.) are modeled
/// as concrete, state-owning subsystems rather than `*Ops` facades, because they
/// directly own their in-memory storage and semantics.
///
/// `MetricsOps` exists solely as a convenience aggregator to provide a stable,
/// import-friendly snapshot surface for callers that need multiple metrics at
/// once, without exposing internal storage details or requiring many imports.
///

pub struct MetricsOps;

impl MetricsOps {
    /// entries
    ///
    /// Project one metrics family into the unified public metrics row shape.
    #[must_use]
    pub fn entries(kind: MetricsKind) -> Vec<MetricEntry> {
        match kind {
            MetricsKind::System => Self::system_entries(),
            MetricsKind::Icc => Self::icc_entries(),
            MetricsKind::Http => Self::http_entries(),
            MetricsKind::Timer => Self::timer_entries(),
            MetricsKind::Access => Self::access_entries(),
            MetricsKind::Delegation => Self::delegation_entries(),
            MetricsKind::RootCapability => Self::root_capability_entries(),
            MetricsKind::CyclesFunding => Self::cycles_funding_entries(),
            MetricsKind::Perf => Self::perf_entries(),
        }
    }

    /// system_entries
    ///
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
                count: Some(count),
                value_u64: None,
                value_u128: None,
            })
            .collect()
    }

    /// icc_entries
    ///
    /// Project inter-canister call counters into the unified public metrics row shape.
    #[must_use]
    fn icc_entries() -> Vec<MetricEntry> {
        IccMetrics::snapshot()
            .entries
            .into_iter()
            .map(|(key, count)| MetricEntry {
                labels: vec![key.method],
                principal: Some(key.target),
                count: Some(count),
                value_u64: None,
                value_u128: None,
            })
            .collect()
    }

    /// http_entries
    ///
    /// Project HTTP outcall counters into the unified public metrics row shape.
    #[must_use]
    fn http_entries() -> Vec<MetricEntry> {
        HttpMetrics::snapshot()
            .entries
            .into_iter()
            .map(|(key, count)| MetricEntry {
                labels: vec![key.method.as_str().to_string(), key.label],
                principal: None,
                count: Some(count),
                value_u64: None,
                value_u128: None,
            })
            .collect()
    }

    /// timer_entries
    ///
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
                count: Some(count),
                value_u64: Some(key.delay_ms),
                value_u128: None,
            })
            .collect()
    }

    /// access_entries
    ///
    /// Project access-denial counters into the unified public metrics row shape.
    #[must_use]
    fn access_entries() -> Vec<MetricEntry> {
        AccessMetrics::snapshot()
            .entries
            .into_iter()
            .map(|(key, count)| MetricEntry {
                labels: vec![key.endpoint, key.kind.as_str().to_string(), key.predicate],
                principal: None,
                count: Some(count),
                value_u64: None,
                value_u128: None,
            })
            .collect()
    }

    /// delegation_entries
    ///
    /// Project delegation authority counters into the unified public metrics row shape.
    #[must_use]
    fn delegation_entries() -> Vec<MetricEntry> {
        DelegationMetrics::snapshot()
            .into_iter()
            .map(|(authority, count)| MetricEntry {
                labels: vec!["delegation_authority".to_string()],
                principal: Some(authority),
                count: Some(count),
                value_u64: None,
                value_u128: None,
            })
            .collect()
    }

    /// root_capability_entries
    ///
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
                    count: Some(count),
                    value_u64: None,
                    value_u128: None,
                },
            )
            .collect()
    }

    /// cycles_funding_entries
    ///
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
                count: None,
                value_u64: None,
                value_u128: Some(cycles),
            })
            .collect()
    }

    /// perf_entries
    ///
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
                    count: Some(entry.count),
                    value_u64: Some(entry.total_instructions),
                    value_u128: None,
                }
            })
            .collect()
    }
}
