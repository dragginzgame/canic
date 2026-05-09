pub mod access;
pub mod auth;
pub mod canister_ops;
pub mod cascade;
pub mod cycles_funding;
pub mod cycles_topup;
pub mod delegated_auth;
pub mod directory;
pub mod http;
pub mod intent;
pub mod inter_canister_call;
pub mod lifecycle;
pub mod management_call;
pub mod platform_call;
pub mod pool;
pub mod provisioning;
pub mod recording;
pub mod replay;
pub mod root_capability;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
pub mod system;
pub mod timer;
pub mod wasm_store;

use crate::{
    dto::metrics::{MetricEntry, MetricValue, MetricsKind},
    perf::{self, PerfKey},
};
use {
    access::AccessMetrics,
    auth::AuthMetrics,
    canister_ops::CanisterOpsMetrics,
    cascade::CascadeMetrics,
    cycles_funding::CyclesFundingMetrics,
    cycles_topup::CyclesTopupMetrics,
    delegated_auth::DelegatedAuthMetrics,
    directory::DirectoryMetrics,
    http::HttpMetrics,
    intent::IntentMetrics,
    inter_canister_call::InterCanisterCallMetrics,
    lifecycle::LifecycleMetrics,
    management_call::ManagementCallMetrics,
    platform_call::PlatformCallMetrics,
    pool::PoolMetrics,
    provisioning::ProvisioningMetrics,
    replay::ReplayMetrics,
    root_capability::RootCapabilityMetrics,
    scaling::ScalingMetrics,
    system::SystemMetrics,
    timer::{TimerMetrics, TimerMode},
    wasm_store::WasmStoreMetrics,
};

#[cfg(feature = "sharding")]
use sharding::ShardingMetrics;

/// Project one metrics family into the unified public metrics row shape.
#[must_use]
pub fn entries(kind: MetricsKind) -> Vec<MetricEntry> {
    match kind {
        MetricsKind::Access => access_entries(),
        MetricsKind::Auth => auth_entries(),
        MetricsKind::CanisterOps => canister_ops_entries(),
        MetricsKind::Cascade => cascade_entries(),
        MetricsKind::CyclesFunding => cycles_funding_entries(),
        MetricsKind::CyclesTopup => cycles_topup_entries(),
        MetricsKind::DelegatedAuth => delegated_auth_entries(),
        MetricsKind::Directory => directory_entries(),
        MetricsKind::Http => http_entries(),
        MetricsKind::Intent => intent_entries(),
        MetricsKind::InterCanisterCall => inter_canister_call_entries(),
        MetricsKind::Lifecycle => lifecycle_entries(),
        MetricsKind::ManagementCall => management_call_entries(),
        MetricsKind::Perf => perf_entries(),
        MetricsKind::PlatformCall => platform_call_entries(),
        MetricsKind::Pool => pool_entries(),
        MetricsKind::Provisioning => provisioning_entries(),
        MetricsKind::Replay => replay_entries(),
        MetricsKind::RootCapability => root_capability_entries(),
        MetricsKind::Scaling => scaling_entries(),
        #[cfg(feature = "sharding")]
        MetricsKind::Sharding => sharding_entries(),
        MetricsKind::System => system_entries(),
        MetricsKind::Timer => timer_entries(),
        MetricsKind::WasmStore => wasm_store_entries(),
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    AccessMetrics::reset();
    AuthMetrics::reset();
    CanisterOpsMetrics::reset();
    CascadeMetrics::reset();
    CyclesFundingMetrics::reset();
    CyclesTopupMetrics::reset();
    DelegatedAuthMetrics::reset();
    DirectoryMetrics::reset();
    HttpMetrics::reset();
    PlatformCallMetrics::reset();
    InterCanisterCallMetrics::reset();
    IntentMetrics::reset();
    LifecycleMetrics::reset();
    ManagementCallMetrics::reset();
    PoolMetrics::reset();
    ProvisioningMetrics::reset();
    ReplayMetrics::reset();
    RootCapabilityMetrics::reset();
    ScalingMetrics::reset();
    #[cfg(feature = "sharding")]
    ShardingMetrics::reset();
    SystemMetrics::reset();
    TimerMetrics::reset();
    WasmStoreMetrics::reset();
    perf::reset();
}

/// Project management-canister call outcome counters into public metrics rows.
#[must_use]
fn management_call_entries() -> Vec<MetricEntry> {
    ManagementCallMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project low-cardinality platform call outcome counters into public metrics rows.
#[must_use]
fn platform_call_entries() -> Vec<MetricEntry> {
    PlatformCallMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.surface.metric_label().to_string(),
                key.mode.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project intent reservation counters into the unified public metrics row shape.
#[must_use]
fn intent_entries() -> Vec<MetricEntry> {
    IntentMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.surface.metric_label().to_string(),
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project provisioning workflow counters into the unified public metrics row shape.
#[must_use]
fn provisioning_entries() -> Vec<MetricEntry> {
    ProvisioningMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.role,
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project replay safety counters into the unified public metrics row shape.
#[must_use]
fn replay_entries() -> Vec<MetricEntry> {
    ReplayMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project auth runtime counters into the unified public metrics row shape.
#[must_use]
fn auth_entries() -> Vec<MetricEntry> {
    AuthMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.surface.metric_label().to_string(),
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project directory placement counters into the unified public metrics row shape.
#[must_use]
fn directory_entries() -> Vec<MetricEntry> {
    DirectoryMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project scaling workflow counters into the unified public metrics row shape.
#[must_use]
fn scaling_entries() -> Vec<MetricEntry> {
    ScalingMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project pool operation counters into the unified public metrics row shape.
#[must_use]
fn pool_entries() -> Vec<MetricEntry> {
    PoolMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project sharding placement counters into the unified public metrics row shape.
#[cfg(feature = "sharding")]
#[must_use]
fn sharding_entries() -> Vec<MetricEntry> {
    ShardingMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project cascade counters into the unified public metrics row shape.
#[must_use]
fn cascade_entries() -> Vec<MetricEntry> {
    CascadeMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.snapshot.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project canister operation counters into the unified public metrics row shape.
#[must_use]
fn canister_ops_entries() -> Vec<MetricEntry> {
    CanisterOpsMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.role,
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project wasm-store operation counters into the unified public metrics row shape.
#[must_use]
fn wasm_store_entries() -> Vec<MetricEntry> {
    WasmStoreMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.source.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
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
fn inter_canister_call_entries() -> Vec<MetricEntry> {
    InterCanisterCallMetrics::snapshot()
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

/// Project delegated-auth counters into the unified public metrics row shape.
#[must_use]
fn delegated_auth_entries() -> Vec<MetricEntry> {
    let mut entries: Vec<_> = DelegatedAuthMetrics::snapshot()
        .into_iter()
        .map(|(authority, count)| MetricEntry {
            labels: vec!["delegated_auth_authority".to_string()],
            principal: Some(authority),
            value: MetricValue::Count(count),
        })
        .collect();

    entries.extend(
        DelegatedAuthMetrics::event_snapshot()
            .into_iter()
            .map(|(key, count)| MetricEntry {
                labels: vec![
                    key.operation.metric_label().to_string(),
                    key.outcome.metric_label().to_string(),
                    key.reason.metric_label().to_string(),
                ],
                principal: None,
                value: MetricValue::Count(count),
            }),
    );

    entries
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

#[cfg(test)]
mod tests;
